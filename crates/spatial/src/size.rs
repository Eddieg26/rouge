#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }

    pub fn scale(&self, scale_factor: f32) -> Self {
        Self {
            width: self.width * scale_factor,
            height: self.height * scale_factor,
        }
    }

    pub fn scale_to_fit(&self, target: Self) -> f32 {
        target.width / self.width.min(target.height / self.aspect_ratio())
    }

    pub fn scale_to_fill(&self, target: Self) -> f32 {
        target.width / self.width.max(target.height / self.aspect_ratio())
    }

    pub fn scale_to_cover(&self, target: Self) -> f32 {
        target.width / self.width.max(target.height / self.height)
    }
}
