use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub center: Vec3,
    pub half_size: Vec3,
}

impl Aabb {
    pub const MAX: Self = Self::new(Vec3::ZERO, Vec3::splat(f32::MAX));
    pub const MIN: Self = Self::new(Vec3::ZERO, Vec3::splat(f32::MIN));

    pub const fn new(center: Vec3, half_size: Vec3) -> Self {
        Self { center, half_size }
    }
}
