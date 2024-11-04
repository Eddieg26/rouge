use glam::Vec3;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sphere {
    pub position: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub const ZERO: Self = Self {
        position: Vec3::ZERO,
        radius: 0.0,
    };

    pub fn new(position: Vec3, radius: f32) -> Self {
        Self { position, radius }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.position).length_squared() <= self.radius * self.radius
    }

    pub fn contains(&self, other: &Sphere) -> bool {
        (self.position - other.position).length() + other.radius <= self.radius
    }

    pub fn intersects(&self, other: &Sphere) -> bool {
        (self.position - other.position).length() <= self.radius + other.radius
    }

    pub fn transform(&self, transform: glam::Mat4) -> Self {
        let position = transform.transform_point3(self.position);
        let scale = transform.to_scale_rotation_translation().0;
        let radius = self.radius * scale.max_element();

        Self { position, radius }
    }
}
