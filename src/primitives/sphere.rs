pub struct Sphere {
    pub center: glam::Vec3,
    pub size: f32,
}

impl Sphere {
    pub const ZERO: Sphere = Sphere {
        center: glam::Vec3::ZERO,
        size: 0.0,
    };

    pub const MAX: Sphere = Sphere {
        center: glam::Vec3::ZERO,
        size: f32::MAX,
    };

    pub fn new(center: glam::Vec3, size: f32) -> Sphere {
        Sphere { center, size }
    }
}
