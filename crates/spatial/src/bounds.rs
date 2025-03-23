use glam::{Vec2, Vec3, Vec3A, Vec4, Vec4Swizzles};

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

    pub fn center(&self) -> Vec3 {
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

    pub fn relative_radius(&self, normal: Vec3A) -> f32 {
        let radius = Vec3A::from(self.size() * 0.5);
        radius.abs().dot(normal).abs()
    }

    pub fn points(&self) -> [Vec3; 8] {
        [
            self.min,
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
            self.max,
        ]
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

impl From<&[Vec2]> for BoundingBox {
    fn from(vertices: &[Vec2]) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for vertex in vertices {
            min = min.min(Vec3::new(vertex.x, vertex.y, 0.0));
            max = max.max(Vec3::new(vertex.x, vertex.y, 0.0));
        }

        Self { min, max }
    }
}

impl From<&[Vec4]> for BoundingBox {
    fn from(vertices: &[Vec4]) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for vertex in vertices {
            min = min.min(vertex.xyz());
            max = max.max(vertex.xyz());
        }

        Self { min, max }
    }
}

impl From<&[f32]> for BoundingBox {
    fn from(vertices: &[f32]) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for vertex in vertices.chunks(3) {
            let vertex = Vec3::new(vertex[0], vertex[1], vertex[2]);
            min = min.min(vertex);
            max = max.max(vertex);
        }

        Self { min, max }
    }
}
