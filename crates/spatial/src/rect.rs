use glam::Vec2;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub position: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub const ZERO: Self = Self {
        position: Vec2::ZERO,
        size: Vec2::ZERO,
    };

    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self { position, size }
    }

    pub fn mid(&self) -> Vec2 {
        self.position + self.size / 2.0
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x <= self.position.x + self.size.x
            && point.y <= self.position.y + self.size.y
    }

    pub fn contains(&self, other: &Rect) -> bool {
        self.contains_point(other.position) && self.contains_point(other.position + other.size)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.position.x <= other.position.x + other.size.x
            && self.position.x + self.size.x >= other.position.x
            && self.position.y <= other.position.y + other.size.y
            && self.position.y + self.size.y >= other.position.y
    }

    pub fn transform(&self, transform: glam::Mat3) -> Self {
        let min = transform.transform_point2(self.position);
        let max = transform.transform_point2(self.position + self.size);

        Self {
            position: min.min(max),
            size: min.max(max) - min,
        }
    }
}
