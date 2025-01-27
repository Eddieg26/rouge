use spatial::rect::Rect;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Viewport {
    pub fn new(x: f32, y: f32, width: f32, height: f32, depth: Range<f32>) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: depth.start,
            max_depth: depth.end,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl From<(Rect, Range<f32>)> for Viewport {
    fn from((rect, depth): (Rect, Range<f32>)) -> Self {
        Self::new(rect.x, rect.y, rect.width, rect.height, depth)
    }
}

impl From<(&Rect, Range<f32>)> for Viewport {
    fn from((rect, depth): (&Rect, Range<f32>)) -> Self {
        Self::new(rect.x, rect.y, rect.width, rect.height, depth)
    }
}
