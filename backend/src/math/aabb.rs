use glam::IVec3;

#[derive(Debug, Clone, Default)]
pub struct AABB<N> {
    pub min: N,
    pub max: N,
}

impl AABB<IVec3> {
    pub fn add(&mut self, position: IVec3) {
        if position.x < self.min.x {
            self.min.x = position.x;
        }
        if position.y < self.min.y {
            self.min.y = position.y;
        }
        if position.z < self.min.z {
            self.min.z = position.z;
        }
        if position.x > self.max.x {
            self.max.x = position.x;
        }
        if position.y > self.max.y {
            self.max.y = position.y;
        }
        if position.z > self.max.z {
            self.max.z = position.z;
        }
    }
}
