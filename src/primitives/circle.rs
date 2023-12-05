pub struct Circle {
    pub center: glam::Vec2,

    // circle diameter
    pub size: f32,
}

impl Circle {
    pub const ZERO: Circle = Circle {
        center: glam::Vec2::ZERO,
        size: 0.0,
    };

    pub const MAX: Circle = Circle {
        center: glam::Vec2::MAX,
        size: f32::MAX,
    };

    pub fn new(center: glam::Vec2, size: f32) -> Circle {
        Circle { center, size }
    }
}
