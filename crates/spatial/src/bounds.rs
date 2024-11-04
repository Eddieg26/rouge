use glam::Vec3;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub const ZERO: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn mid(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }

    pub fn contains(&self, other: &BoundingBox) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn transform(&self, transform: glam::Mat4) -> Self {
        let min = transform.transform_point3(self.min);
        let max = transform.transform_point3(self.max);

        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }
}

impl From<&[Vec3]> for BoundingBox {
    fn from(vertices: &[Vec3]) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for vertex in vertices {
            min = min.min(*vertex);
            max = max.max(*vertex);
        }

        Self { min, max }
    }
}
