use glam::Vec3;

pub struct Aabb {
    pub center: Vec3,
    pub half_size: Vec3,
}

impl Aabb {
    pub const fn new(center: Vec3, half_size: Vec3) -> Self {
        Self { center, half_size }
    }
}
