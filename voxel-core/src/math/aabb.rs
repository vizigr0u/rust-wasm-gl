use glam::IVec3;

#[derive(Debug, Clone, Default)]
pub struct AABB<N> {
    pub min: N,
    pub max: N,
}

impl AABB<IVec3> {
    /// Empty box (min=MAX, max=MIN): the first `add` sets both corners.
    pub fn empty() -> Self {
        Self {
            min: IVec3::MAX,
            max: IVec3::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_box_does_not_include_origin() {
        let mut b = AABB::empty();
        assert!(b.is_empty());
        b.add(IVec3::new(5, 6, 7));
        assert!(!b.is_empty());
        assert_eq!(b.min, IVec3::new(5, 6, 7));
        assert_eq!(b.max, IVec3::new(5, 6, 7));
    }
}
