use asset::AssetId;
use ecs::core::component::Component;
use graphics::{Color, View, Viewport};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearFlag {
    Skybox,
    Color(Color),
}

impl From<Color> for ClearFlag {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
    Perspective {
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub clear: Option<ClearFlag>,
    pub viewport: Viewport,
    pub projection: Projection,
    pub target: Option<AssetId>,
    pub depth: u32,
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_clear(mut self, clear: impl Into<ClearFlag>) -> Self {
        self.clear = Some(clear.into());
        self
    }

    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.projection = projection;
        self
    }

    pub fn with_target(mut self, target: AssetId) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            clear: None,
            viewport: Viewport::default(),
            projection: Projection::Perspective {
                fov: 27.0,
                aspect: 1.0,
                near: 0.3,
                far: 1000.0,
            },
            target: None,
            depth: 0,
        }
    }
}

impl Component for Camera {}

impl View for Camera {
    fn sort(&self, other: &Self) -> std::cmp::Ordering {
        self.depth.cmp(&other.depth)
    }
}
