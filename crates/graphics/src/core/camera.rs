use super::{Color, RenderAsset};
use asset::AssetId;
use spatial::size::Size;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClearFlag {
    Skybox,
    Color(Color),
}

impl From<Color> for ClearFlag {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Viewport {
    pub position: Size,
    pub size: glam::Vec2,
    pub depth: i32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            position: Size::ZERO,
            size: glam::Vec2::new(1.0, 1.0),
            depth: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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

impl Projection {
    pub fn matrix(&self) -> glam::Mat4 {
        match self {
            Self::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => glam::Mat4::orthographic_rh(*left, *right, *bottom, *top, *near, *far),
            Self::Perspective {
                fov,
                aspect,
                near,
                far,
            } => glam::Mat4::perspective_rh(fov.to_radians(), *aspect, *near, *far),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct CameraData {
    pub world: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
    pub projection_inv: glam::Mat4,
    pub position: glam::Vec3,

    #[serde(skip)]
    pub _padding: f32,
}

impl CameraData {
    pub fn new(world: glam::Mat4, projection: glam::Mat4) -> Self {
        let view = world.inverse();
        let projection_inv = projection.inverse();
        let position = view.w_axis.truncate();
        Self {
            world,
            view,
            projection,
            projection_inv,
            position,
            _padding: 0.0,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct RenderDepth(pub u32);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenderCamera {
    pub clear: Option<ClearFlag>,
    pub viewport: Viewport,
    pub projection: Projection,
    pub target: Option<AssetId>,
    pub depth: RenderDepth,
    pub data: CameraData,
}

impl Default for RenderCamera {
    fn default() -> Self {
        Self {
            clear: Some(ClearFlag::Color(Color::black())),
            viewport: Viewport::default(),
            projection: Projection::Orthographic {
                left: 0.0,
                right: 1.0,
                bottom: 0.0,
                top: 1.0,
                near: -1.0,
                far: 1.0,
            },
            target: None,
            depth: RenderDepth(0),
            data: CameraData::default(),
        }
    }
}

impl RenderAsset for RenderCamera {
    type Id = RenderDepth;
}
