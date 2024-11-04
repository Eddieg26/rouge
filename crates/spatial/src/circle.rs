use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub const ZERO: Self = Self {
        center: Vec2::ZERO,
        radius: 0.0,
    };

    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    pub fn contains(&self, other: &Circle) -> bool {
        (self.center - other.center).length() + other.radius <= self.radius
    }

    pub fn intersects(&self, other: &Circle) -> bool {
        (self.center - other.center).length() <= self.radius + other.radius
    }

    pub fn transform(&self, transform: glam::Mat3) -> Self {
        let position = transform.transform_point2(self.center);
        let scale_x = (transform.x_axis.x * transform.x_axis.x
            + transform.y_axis.x * transform.y_axis.x)
            .sqrt();
        let scale_y = (transform.x_axis.y * transform.x_axis.y
            + transform.y_axis.y * transform.y_axis.y)
            .sqrt();

        let scale = Vec2::new(scale_x, scale_y);
        let radius = self.radius * scale.max_element();

        Self {
            center: position,
            radius,
        }
    }
}
