use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Side {
    Top = 0,
    Bottom,
    Front,
    Back,
    Right,
    Left,
}

pub const SIDE_NORMS: [Vec3; 6] = [
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
    Vec3::X,
    Vec3::NEG_X,
];

impl From<Side> for Vec3 {
    fn from(val: Side) -> Self {
        SIDE_NORMS[val as usize]
    }
}
