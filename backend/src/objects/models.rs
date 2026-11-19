use glam::{Mat4, Vec3};

use crate::world::{ChunkPos, WorldPos};

use super::{Player, PLAYER_SCALE};

const GIZMO_SCALE: f32 = 10.0;

/// Model matrix of the player cube, relative to the player's chunk.
pub fn player_model(player: &Player, player_chunk: ChunkPos) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::splat(PLAYER_SCALE),
        glam::Quat::IDENTITY,
        player.get_position().to_screen_pos(player_chunk),
    )
}

/// Model matrix of the world-origin gizmo, relative to the player's chunk.
pub fn gizmo_model(player_chunk: ChunkPos) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::splat(GIZMO_SCALE),
        glam::Quat::IDENTITY,
        WorldPos::ZERO.to_screen_pos(player_chunk),
    )
}
