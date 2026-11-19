//! GPU-side frame timing, for profiling. Two independent readings, both async:
//! - timestamp queries around the main pass (needs `TIMESTAMP_QUERY`; Chrome only exposes it
//!   with `--enable-dawn-features=allow_unsafe_apis`),
//! - submit -> `on_submitted_work_done` wall clock (works everywhere, includes queue latency).
//!
//! Each keeps one request in flight, so a reading lands every few frames and is reported by
//! `take_*` exactly once (NaN otherwise). That keeps the per-frame cost near zero and means
//! consecutive samples are never duplicates.

use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering::Relaxed},
    Arc,
};

const IDLE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;

struct Timestamps {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    staging: wgpu::Buffer,
    period_ns: f32,
    state: Arc<AtomicU8>,
    /// The main pass of this frame carries the timestamp writes.
    armed: bool,
}

pub struct GpuProbe {
    ts: Option<Timestamps>,
    done_state: Arc<AtomicU8>,
    done_ms: Arc<AtomicU64>,
}

impl GpuProbe {
    pub const FEATURE: wgpu::Features = wgpu::Features::TIMESTAMP_QUERY;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let ts = device.features().contains(Self::FEATURE).then(|| {
            let buf = |label, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: 16,
                    usage,
                    mapped_at_creation: false,
                })
            };
            Timestamps {
                set: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("main pass timestamps"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 2,
                }),
                resolve: buf(
                    "ts resolve",
                    wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                ),
                staging: buf(
                    "ts staging",
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                ),
                period_ns: queue.get_timestamp_period(),
                state: Arc::new(AtomicU8::new(IDLE)),
                armed: false,
            }
        });
        Self {
            ts,
            done_state: Arc::new(AtomicU8::new(IDLE)),
            done_ms: Arc::new(AtomicU64::new(f64::NAN.to_bits())),
        }
    }

    /// A readback is still in flight.
    pub fn busy(&self) -> bool {
        self.done_state.load(Relaxed) == PENDING
            || self
                .ts
                .as_ref()
                .is_some_and(|t| t.state.load(Relaxed) == PENDING)
    }

    pub fn has_timestamps(&self) -> bool {
        self.ts.is_some()
    }

    /// Collect a finished timestamp readback (ms), if one landed.
    pub fn take_pass_ms(&mut self) -> f64 {
        let Some(ts) = &self.ts else { return f64::NAN };
        if ts.state.load(Relaxed) != READY {
            return f64::NAN;
        }
        let ms = match ts.staging.slice(..).get_mapped_range() {
            Ok(data) => {
                let t: &[u64] = bytemuck::cast_slice(&data);
                match (t.first(), t.get(1)) {
                    (Some(&a), Some(&b)) if b >= a => (b - a) as f64 * ts.period_ns as f64 * 1e-6,
                    _ => f64::NAN,
                }
            }
            Err(_) => f64::NAN,
        };
        ts.staging.unmap();
        ts.state.store(IDLE, Relaxed);
        ms
    }

    pub fn take_done_ms(&mut self) -> f64 {
        if self.done_state.load(Relaxed) != READY {
            return f64::NAN;
        }
        self.done_state.store(IDLE, Relaxed);
        f64::from_bits(self.done_ms.load(Relaxed))
    }

    /// Timestamp writes for this frame's main pass, if a readback slot is free.
    pub fn pass_writes(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let ts = self.ts.as_mut()?;
        ts.armed = ts.state.load(Relaxed) == IDLE;
        ts.armed.then(|| wgpu::RenderPassTimestampWrites {
            query_set: &ts.set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        })
    }

    /// Call after the main pass has ended, before `finish`.
    pub fn resolve(&mut self, enc: &mut wgpu::CommandEncoder) {
        let Some(ts) = self.ts.as_ref().filter(|t| t.armed) else {
            return;
        };
        enc.resolve_query_set(&ts.set, 0..2, &ts.resolve, 0);
        enc.copy_buffer_to_buffer(&ts.resolve, 0, &ts.staging, 0, 16);
    }

    /// Call right after `queue.submit`.
    pub fn after_submit(&mut self, queue: &wgpu::Queue, submit_t: f64) {
        if let Some(ts) = self.ts.as_mut().filter(|t| t.armed) {
            ts.armed = false;
            ts.state.store(PENDING, Relaxed);
            let state = Arc::clone(&ts.state);
            ts.staging
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    state.store(if r.is_ok() { READY } else { IDLE }, Relaxed);
                });
        }
        if self.done_state.load(Relaxed) == IDLE {
            self.done_state.store(PENDING, Relaxed);
            let (state, ms) = (self.done_state.clone(), self.done_ms.clone());
            queue.on_submitted_work_done(move || {
                ms.store(
                    (crate::utils::performance_now() - submit_t).to_bits(),
                    Relaxed,
                );
                state.store(READY, Relaxed);
            });
        }
    }
}
