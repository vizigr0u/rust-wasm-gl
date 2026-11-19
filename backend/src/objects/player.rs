use glam::{Vec2, Vec3};
use voxel_core::playerview::{PlayerView, LOOK_SPEED_DEG_PER_S};

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
    view: PlayerView,
    /// (forward/back, right/left), each -1..=1.
    move_axis: Vec2,
    /// (yaw, pitch) input, each -1..=1.
    look_axis: Vec2,
}

impl Player {
    pub fn new(origin: Vec3) -> Player {
        Player {
            position: WorldPos::ZERO + origin,
            ..Default::default()
        }
    }

    pub fn view(&self) -> &PlayerView {
        &self.view
    }

    pub fn view_mut(&mut self) -> &mut PlayerView {
        &mut self.view
    }

    /// Fly velocity: W/S along the view (with pitch), A/D along the yaw-only right axis.
    fn fly_velocity(view: &PlayerView, axis: Vec2) -> Vec3 {
        (view.forward() * axis.x + view.right() * axis.y).normalize_or_zero() * MAX_SPEED
    }

    pub fn update(&mut self, time: &Time) {
        if time.delta_time() <= 0.0 {
            return;
        }
        let dt = (time.delta_time() as f32).min(MAX_DELTA_MS) * 0.001;
        // Rotate first so movement uses this frame's view.
        let turn = (LOOK_SPEED_DEG_PER_S * dt).to_radians();
        self.view
            .rotate(self.look_axis.x * turn, self.look_axis.y * turn);
        self.velocity = Self::fly_velocity(&self.view, self.move_axis);
        if self.velocity != Vec3::ZERO {
            self.position += self.velocity * dt;
        }
    }

    /// Teleport by `delta` (profiling hooks drive deterministic streaming with this).
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
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
        self.move_axis = Vec2::new(axis("KeyW", "KeyS"), axis("KeyD", "KeyA"));
        self.look_axis = Vec2::new(
            axis("ArrowRight", "ArrowLeft"),
            axis("ArrowUp", "ArrowDown"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_is_normalized() {
        let v = PlayerView::new();
        let vel = Player::fly_velocity(&v, Vec2::new(1.0, 1.0));
        assert!((vel.length() - MAX_SPEED).abs() < 1e-4);
        assert_eq!(Player::fly_velocity(&v, Vec2::ZERO), Vec3::ZERO);
    }

    #[test]
    fn default_view_moves_plus_x_and_strafes_plus_z() {
        let v = PlayerView::new();
        assert!(
            (Player::fly_velocity(&v, Vec2::new(1.0, 0.0)) - Vec3::X * MAX_SPEED).length() < 1e-4
        );
        assert!(
            (Player::fly_velocity(&v, Vec2::new(0.0, 1.0)) - Vec3::Z * MAX_SPEED).length() < 1e-4
        );
    }
}
