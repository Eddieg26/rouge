#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self::splat(0.0);

    pub const MIN: Self = Self::splat(f32::MIN);

    pub const MAX: Self = Self::splat(f32::MAX);

    pub const fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl AsRef<glam::Vec2> for Point {
    fn as_ref(&self) -> &glam::Vec2 {
        unsafe { std::mem::transmute(self) }
    }
}

impl AsMut<glam::Vec2> for Point {
    fn as_mut(&mut self) -> &mut glam::Vec2 {
        unsafe { std::mem::transmute(self) }
    }
}

impl From<glam::Vec2> for Point {
    fn from(value: glam::Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl Into<glam::Vec2> for Point {
    fn into(self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }
}
