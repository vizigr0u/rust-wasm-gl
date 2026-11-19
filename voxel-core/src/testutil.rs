//! Shared helpers for snapshot tests.
use glam::{Mat4, Vec3};

/// FNV-1a over i32 words: stable across platforms, enough for snapshot hashes.
pub fn hash_words(words: &[i32]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in words {
        for b in w.to_le_bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

pub fn fmt_vec3(v: Vec3) -> String {
    format!("[{:.5}, {:.5}, {:.5}]", v.x, v.y, v.z)
}

pub fn fmt_mat4(m: Mat4) -> Vec<String> {
    (0..4)
        .map(|r| {
            let row = m.row(r);
            format!("[{:.5}, {:.5}, {:.5}, {:.5}]", row.x, row.y, row.z, row.w)
        })
        .collect()
}
