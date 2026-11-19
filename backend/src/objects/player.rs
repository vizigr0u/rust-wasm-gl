use glam::Vec3;

use crate::{
    core::{HandleInputs, Time},
    world::WorldPos,
};

const MAX_SPEED: f32 = 10.0;
/// Clamp dt (ms) so a hidden tab doesn't teleport the player.
const MAX_DELTA_MS: f32 = 100.0;
pub const PLAYER_SCALE: f32 = 0.5;

/// Plain simulation data. No GL, available from the first frame.
#[derive(Debug, Default)]
pub struct Player {
    position: WorldPos,
    velocity: Vec3,
}

impl Player {
    pub fn new(origin: Vec3) -> Player {
        Player {
            position: WorldPos::ZERO + origin,
            velocity: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, time: &Time) {
        if time.delta_time() > 0.0 && self.velocity != Vec3::ZERO {
            let dt = (time.delta_time() as f32).min(MAX_DELTA_MS);
            self.position += self.velocity * dt * 0.001;
        }
    }

    /// Chunk-local offset of the player.
    pub fn get_screen_position(&self) -> Vec3 {
        self.position.offset()
    }

    pub fn get_position(&self) -> WorldPos {
        self.position
    }
}

impl HandleInputs for Player {
    fn handle_inputs(&mut self, inputs: &crate::core::InputState) {
        let axis = |pos: &str, neg: &str| {
            inputs.is_key_down(pos) as i32 as f32 - inputs.is_key_down(neg) as i32 as f32
        };
        let velocity = Vec3::new(axis("KeyW", "KeyS"), 0.0, axis("KeyD", "KeyA"));
        self.velocity = velocity.normalize_or_zero() * MAX_SPEED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_is_normalized() {
        let v = Vec3::new(1.0, 0.0, 1.0).normalize_or_zero() * MAX_SPEED;
        assert!((v.length() - MAX_SPEED).abs() < 1e-4);
        assert_eq!(Vec3::ZERO.normalize_or_zero(), Vec3::ZERO);
    }
}
