use spatial::rect::Rect;
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub depth: Range<f32>,
}

impl Viewport {
    pub fn new(x: f32, y: f32, width: f32, height: f32, depth: Range<f32>) -> Self {
        Self {
            x,
            y,
            width,
            height,
            depth,
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
            depth: 0.0..1.0,
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
