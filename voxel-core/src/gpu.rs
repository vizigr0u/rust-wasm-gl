//! Pure GPU-side bookkeeping: arena range allocator and merged draw list.
use std::ops::Range;

use crate::world::ChunkPos;

/// Ranges are rounded up to this many quads.
pub const ARENA_ALIGN: u32 = 64;

/// First-fit allocator over `0..capacity` quads. Frees merge with neighbours.
#[derive(Debug, Clone)]
pub struct RangeAllocator {
    /// Sorted, non-overlapping, non-adjacent free ranges.
    free: Vec<Range<u32>>,
    capacity: u32,
}

impl RangeAllocator {
    pub fn new(capacity: u32) -> Self {
        Self {
            free: if capacity > 0 {
                vec![0..capacity]
            } else {
                vec![]
            },
            capacity,
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Size actually reserved for a request of `n` quads.
    pub fn rounded(n: u32) -> u32 {
        n.div_ceil(ARENA_ALIGN) * ARENA_ALIGN
    }

    pub fn alloc(&mut self, n: u32) -> Option<Range<u32>> {
        let n = Self::rounded(n.max(1));
        let i = self.free.iter().position(|r| r.end - r.start >= n)?;
        let r = self.free[i].clone();
        if r.end - r.start == n {
            self.free.remove(i);
        } else {
            self.free[i].start += n;
        }
        Some(r.start..r.start + n)
    }

    pub fn free(&mut self, range: Range<u32>) {
        debug_assert!(range.end <= self.capacity && range.start < range.end);
        let i = self.free.partition_point(|r| r.start < range.start);
        debug_assert!(i == 0 || self.free[i - 1].end <= range.start, "double free");
        debug_assert!(i == self.free.len() || range.end <= self.free[i].start);
        self.free.insert(i, range);
        // merge with next, then previous
        if i + 1 < self.free.len() && self.free[i].end == self.free[i + 1].start {
            self.free[i].end = self.free[i + 1].end;
            self.free.remove(i + 1);
        }
        if i > 0 && self.free[i - 1].end == self.free[i].start {
            self.free[i - 1].end = self.free[i].end;
            self.free.remove(i);
        }
    }

    /// Extend capacity; the new tail is free.
    pub fn grow(&mut self, new_capacity: u32) {
        assert!(new_capacity >= self.capacity);
        let old = self.capacity;
        self.capacity = new_capacity;
        if new_capacity > old {
            self.free(old..new_capacity);
        }
    }

    pub fn used(&self) -> u32 {
        self.capacity - self.free.iter().map(|r| r.end - r.start).sum::<u32>()
    }

    pub fn largest_free(&self) -> u32 {
        self.free.iter().map(|r| r.end - r.start).max().unwrap_or(0)
    }
}

/// Hands out small stable `u32` slots, lowest free first. Grows on demand; freed slots are reused.
#[derive(Debug, Default, Clone)]
pub struct SlotAllocator {
    /// Free slots, kept sorted descending so `pop` gives the lowest.
    free: Vec<u32>,
    /// One past the highest slot ever handed out.
    next: u32,
}

impl SlotAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self) -> u32 {
        if let Some(s) = self.free.pop() {
            return s;
        }
        self.next += 1;
        self.next - 1
    }

    pub fn free(&mut self, slot: u32) {
        debug_assert!(slot < self.next, "slot never allocated");
        debug_assert!(!self.free.contains(&slot), "double free");
        let i = self.free.partition_point(|&f| f > slot);
        self.free.insert(i, slot);
    }

    /// Slots `0..capacity()` have been handed out at some point (buffers must cover them).
    pub fn capacity(&self) -> u32 {
        self.next
    }

    pub fn in_use(&self) -> u32 {
        self.next - self.free.len() as u32
    }
}

/// Longest run of quads in one draw; the static index buffer covers this many.
pub const MAX_QUADS_PER_DRAW: u32 = 4096;

/// Index buffer for `max_quads` quads: `4k + {0,1,2, 2,1,3}` (the strip `0,1,2,3` as triangles).
pub fn quad_indices(max_quads: u32) -> Vec<u16> {
    assert!(max_quads * 4 <= 1 << 16, "u16 index overflow");
    (0..max_quads as u16)
        .flat_map(|k| [0u16, 1, 2, 2, 1, 3].map(|i| 4 * k + i))
        .collect()
}

/// One draw of quads `first..first+count`, with draw slot `slot` (`chunk_slot * 6 + side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawEntry {
    pub slot: u32,
    pub first: u32,
    pub count: u32,
}

/// Draw entries; adjacent ones in the same slot merge into one, never above `MAX_QUADS_PER_DRAW`.
#[derive(Debug, Default)]
pub struct DrawList {
    pub entries: Vec<DrawEntry>,
    /// Pushes before merging.
    pub pushed: u32,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pushed = 0;
    }

    pub fn push(&mut self, mut e: DrawEntry, merge: bool) {
        self.pushed += 1;
        if merge {
            if let Some(p) = self.entries.last_mut() {
                if p.slot == e.slot && p.first + p.count == e.first {
                    let take = e.count.min(MAX_QUADS_PER_DRAW - p.count);
                    p.count += take;
                    e.first += take;
                    e.count -= take;
                    if e.count == 0 {
                        return;
                    }
                }
            }
        }
        while e.count > MAX_QUADS_PER_DRAW {
            self.entries.push(DrawEntry {
                count: MAX_QUADS_PER_DRAW,
                ..e
            });
            e.first += MAX_QUADS_PER_DRAW;
            e.count -= MAX_QUADS_PER_DRAW;
        }
        self.entries.push(e);
    }
}

/// Chunks whose mesh is waiting for a GPU upload, oldest first.
#[derive(Debug, Default)]
pub struct UploadQueue {
    pending: std::collections::VecDeque<ChunkPos>,
}

impl UploadQueue {
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Queue `pos`; a rebuilt chunk already waiting keeps its place.
    pub fn push(&mut self, pos: ChunkPos) {
        if !self.pending.contains(&pos) {
            self.pending.push_back(pos);
        }
    }

    pub fn remove(&mut self, pos: ChunkPos) {
        self.pending.retain(|p| *p != pos);
    }

    /// Pop chunks until `bytes_of` adds up to `budget` bytes. Always pops at least one when
    /// non-empty (a chunk bigger than the budget must still get through).
    pub fn drain_budget(
        &mut self,
        budget: usize,
        mut bytes_of: impl FnMut(ChunkPos) -> usize,
    ) -> Vec<ChunkPos> {
        let mut out = Vec::new();
        let mut used = 0;
        while let Some(pos) = self.pending.front().copied() {
            let n = bytes_of(pos);
            if !out.is_empty() && used + n > budget {
                break;
            }
            used += n;
            out.push(pos);
            self.pending.pop_front();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_free_roundtrip() {
        let mut a = RangeAllocator::new(256);
        let r = a.alloc(10).unwrap();
        assert_eq!(r, 0..64);
        assert_eq!(a.used(), 64);
        a.free(r);
        assert_eq!(a.used(), 0);
        assert_eq!(a.largest_free(), 256);
    }

    #[test]
    fn freed_neighbours_merge() {
        let mut a = RangeAllocator::new(192);
        let (x, y, z) = (
            a.alloc(64).unwrap(),
            a.alloc(64).unwrap(),
            a.alloc(64).unwrap(),
        );
        a.free(x);
        a.free(z);
        assert_eq!(a.largest_free(), 64);
        a.free(y);
        assert_eq!(a.largest_free(), 192);
    }

    #[test]
    fn first_fit_reuses_hole() {
        let mut a = RangeAllocator::new(256);
        let x = a.alloc(64).unwrap();
        let _y = a.alloc(64).unwrap();
        a.free(x.clone());
        assert_eq!(a.alloc(30).unwrap(), x);
    }

    #[test]
    fn full_then_grow() {
        let mut a = RangeAllocator::new(64);
        a.alloc(64).unwrap();
        assert!(a.alloc(1).is_none());
        a.grow(128);
        assert_eq!(a.alloc(1).unwrap(), 64..128);
        assert_eq!(a.capacity(), 128);
    }

    #[test]
    fn grow_merges_with_trailing_free() {
        let mut a = RangeAllocator::new(128);
        a.alloc(64).unwrap();
        a.grow(256);
        assert_eq!(a.largest_free(), 192);
    }

    #[test]
    fn draw_list_merges_adjacent_same_group_only() {
        let mut l = DrawList::default();
        let e = |slot, first, count| DrawEntry { slot, first, count };
        l.push(e(0, 0, 4), true);
        l.push(e(0, 4, 2), true); // adjacent -> merged
        l.push(e(0, 10, 1), true); // gap
        l.push(e(1, 11, 1), true); // other group
        assert_eq!(l.entries, vec![e(0, 0, 6), e(0, 10, 1), e(1, 11, 1)]);
        assert_eq!(l.pushed, 4);
        l.clear();
        l.push(e(0, 0, 4), false);
        l.push(e(0, 4, 2), false);
        assert_eq!(l.entries.len(), 2);
    }

    #[test]
    fn draw_list_splits_long_runs() {
        let m = MAX_QUADS_PER_DRAW;
        let e = |first, count| DrawEntry {
            slot: 3,
            first,
            count,
        };
        let mut l = DrawList::default();
        l.push(e(0, m + 10), true);
        assert_eq!(l.entries, vec![e(0, m), e(m, 10)]);
        l.push(e(m + 10, m), true); // tops up the tail, rest spills
        assert_eq!(l.entries, vec![e(0, m), e(m, m), e(2 * m, 10)]);
        assert!(l.entries.iter().all(|d| d.count <= m));
    }

    #[test]
    fn quad_indices_pattern() {
        let i = quad_indices(2);
        assert_eq!(i, vec![0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7]);
        assert_eq!(
            quad_indices(MAX_QUADS_PER_DRAW).len() as u32,
            6 * MAX_QUADS_PER_DRAW
        );
    }

    #[test]
    fn slots_alloc_free_reuse() {
        let mut a = SlotAllocator::new();
        assert_eq!((a.alloc(), a.alloc(), a.alloc()), (0, 1, 2));
        assert_eq!((a.capacity(), a.in_use()), (3, 3));
        a.free(1);
        a.free(0);
        assert_eq!(a.in_use(), 1);
        assert_eq!(a.alloc(), 0, "lowest free first");
        assert_eq!(a.alloc(), 1);
        assert_eq!(a.alloc(), 3, "grows when none free");
        assert_eq!(a.capacity(), 4);
    }

    #[test]
    fn slots_never_duplicate() {
        let mut a = SlotAllocator::new();
        let mut live = std::collections::HashSet::new();
        for i in 0..500u32 {
            if i % 3 == 2 {
                let s = *live.iter().min().unwrap();
                live.remove(&s);
                a.free(s);
            } else {
                assert!(live.insert(a.alloc()), "duplicate slot");
            }
            assert_eq!(a.in_use() as usize, live.len());
        }
        assert!(live.iter().all(|&s| s < a.capacity()));
    }

    #[test]
    fn upload_queue_budget() {
        use glam::ivec3;
        let p = |x| ChunkPos::from(ivec3(x, 0, 0));
        let mut q = UploadQueue::default();
        for x in [1, 2, 3, 2] {
            q.push(p(x));
        }
        assert_eq!(q.len(), 3, "dedupe");
        q.remove(p(2));
        assert_eq!(
            q.drain_budget(100, |_| 60),
            vec![p(1)],
            "second would bust budget"
        );
        q.push(p(4));
        assert_eq!(
            q.drain_budget(1, |_| 500),
            vec![p(3)],
            "oversized still passes alone"
        );
        assert_eq!(q.drain_budget(1000, |_| 10), vec![p(4)]);
        assert!(q.is_empty());
        assert!(q.drain_budget(10, |_| 1).is_empty());
    }
}
