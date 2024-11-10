#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub const MAX: Self = Self {
        width: u32::MAX,
        height: u32::MAX,
    };

    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn aspect_ratio(&self) -> u32 {
        self.width / self.height
    }

    pub fn scale(&self, scale_factor: u32) -> Self {
        Self {
            width: self.width * scale_factor,
            height: self.height * scale_factor,
        }
    }

    pub fn scale_to_fit(&self, target: Self) -> u32 {
        target.width / self.width.min(target.height / self.aspect_ratio())
    }

    pub fn scale_to_fill(&self, target: Self) -> u32 {
        target.width / self.width.max(target.height / self.aspect_ratio())
    }

    pub fn scale_to_cover(&self, target: Self) -> u32 {
        target.width / self.width.max(target.height / self.height)
    }

    pub fn max(&self, other: Self) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    pub fn min(&self, other: Self) -> Self {
        Self {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }
}
