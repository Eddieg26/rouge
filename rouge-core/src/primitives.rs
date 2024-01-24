

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const ZERO: Self = Self::new(0, 0);

    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const ZERO: Self = Self::new(0, 0, 0, 0);

    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn contains_rect(&self, rect: &Rect) -> bool {
        self.contains_point(rect.x, rect.y)
            && self.contains_point(rect.x + rect.width, rect.y)
            && self.contains_point(rect.x, rect.y + rect.height)
            && self.contains_point(rect.x + rect.width, rect.y + rect.height)
    }

    pub fn intersects(&self, rect: &Rect) -> bool {
        self.contains_point(rect.x, rect.y)
            || self.contains_point(rect.x + rect.width, rect.y)
            || self.contains_point(rect.x, rect.y + rect.height)
            || self.contains_point(rect.x + rect.width, rect.y + rect.height)
    }
}
