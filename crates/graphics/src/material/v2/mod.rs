use std::num::NonZeroU32;

use crate::{
    extract::RenderResourceExtractor,
    resource::{
        BindGroup, BindGroupLayout, BufferData, Mesh, MeshAttributeKind, SubMesh,
        VertexBufferLayout,
    },
};
use asset::{asset::Asset, io::cache::LoadPath, AssetRef};
use spatial::Aabb;
use wgpu::{BlendState, PrimitiveState};

pub trait View: Send + Sync + 'static {
    type Uniform: BufferData + Send + Sync + 'static;
}

pub trait LightBinding: RenderResourceExtractor + Send + Sync + 'static {
    fn bind_group(&self) -> Option<&BindGroup>;
    fn bind_group_layout(&self) -> Option<&BindGroupLayout>;
}

pub trait Material: Asset + Send + Sync + 'static {
    type Light: LightBinding;

    fn mode() -> BlendMode;
    fn shader() -> impl Into<LoadPath>;
}

pub struct DrawPipelineVertexInfo {
    pub vertex_attributes: Vec<VertexAttribute>,
    pub instance_attributes: Vec<VertexAttribute>,
    pub primitive: PrimitiveState,
    pub depth_write: DepthWrite,
    pub shader: LoadPath,
}

pub struct MeshCullData {
    pub aabb: Aabb,
}

pub trait Draw: Send + Sync + 'static {
    type View: View;
    type Mesh: BufferData + Copy + Send + Sync + 'static;
    type Material: Material;

    const BATCH_SIZE: Option<NonZeroU32> = None;
    const CULL: bool = false;
    const SORT: bool = false;

    fn entity(&self) -> ecs::Entity;
    fn material(&self) -> AssetRef<Self::Material>;
    fn mesh(&self) -> AssetRef<Mesh>;
    fn submesh(&self) -> Option<SubMesh> {
        None
    }
    fn data(&self) -> Self::Mesh;

    fn bounds(&self) -> Option<Aabb> {
        None
    }

    fn pipeline_info() -> DrawPipelineVertexInfo;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl Into<BlendState> for BlendMode {
    fn into(self) -> wgpu::BlendState {
        match self {
            Self::Opaque => wgpu::BlendState::REPLACE,
            Self::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    Auto,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexAttribute {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,
}

impl VertexAttribute {
    pub fn size(&self) -> u64 {
        match self {
            Self::Float => 4,
            Self::Vec2 => 8,
            Self::Vec3 => 12,
            Self::Vec4 => 16,
            Self::Color => 4,
        }
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            Self::Float => wgpu::VertexFormat::Float32,
            Self::Vec2 => wgpu::VertexFormat::Float32x2,
            Self::Vec3 => wgpu::VertexFormat::Float32x3,
            Self::Vec4 => wgpu::VertexFormat::Float32x4,
            Self::Color => wgpu::VertexFormat::Float32x4,
        }
    }

    pub fn into_layout(attributes: &[Self], mode: wgpu::VertexStepMode) -> VertexBufferLayout {
        let mut stride = 0;
        let attributes = attributes
            .iter()
            .enumerate()
            .map(|(location, a)| {
                let format = a.format();
                let offset = stride;
                stride += a.size();
                wgpu::VertexAttribute {
                    format,
                    offset,
                    shader_location: location as u32,
                }
            })
            .collect();

        VertexBufferLayout {
            array_stride: stride,
            step_mode: mode,
            attributes,
        }
    }
}

impl From<MeshAttributeKind> for VertexAttribute {
    fn from(value: MeshAttributeKind) -> Self {
        match value {
            MeshAttributeKind::Position => VertexAttribute::Vec3,
            MeshAttributeKind::Normal => VertexAttribute::Vec3,
            MeshAttributeKind::TexCoord0 => VertexAttribute::Vec2,
            MeshAttributeKind::TexCoord1 => VertexAttribute::Vec2,
            MeshAttributeKind::Tangent => VertexAttribute::Vec3,
            MeshAttributeKind::Color => VertexAttribute::Color,
        }
    }
}
