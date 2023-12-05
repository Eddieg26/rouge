pub struct Plane {
    pub normal: glam::Vec3,
    pub distance: f32,
}

impl Plane {
    pub const ZERO: Plane = Plane {
        normal: glam::Vec3::ZERO,
        distance: 0.0,
    };

    pub const MAX: Plane = Plane {
        normal: glam::Vec3::ZERO,
        distance: f32::MAX,
    };

    pub fn new(normal: glam::Vec3, distance: f32) -> Plane {
        Plane { normal, distance }
    }
}
