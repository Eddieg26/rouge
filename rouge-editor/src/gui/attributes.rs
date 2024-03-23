use super::core::{Position, Size};
use rouge_graphics::core::ty::color::Color;

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    position: Position,
    size: Size,
    color: Color,
}

impl Attributes {
    pub fn new(size: Size) -> Self {
        Self {
            position: Position::Relative { x: 0, y: 0 },
            size,
            color: Color::white(),
        }
    }

    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn color(&self) -> &Color {
        &self.color
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Properties {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

impl Properties {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Color) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
        }
    }
}

impl Default for Properties {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            color: Color::white(),
        }
    }
}
