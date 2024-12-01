use super::{
    BindGroup, BindGroupLayout, CreateBindGroup, FragmentState, MeshAttributeKind, RenderPipeline,
    RenderPipelineDesc, Shader, VertexBufferLayout, VertexState,
};
use crate::{RenderAsset, RenderDevice, RenderResourceExtractor};
use asset::{io::cache::LoadPath, Asset, AssetId};
use ecs::{
    core::{resource::Resource, IndexMap, Type},
    system::unlifetime::ReadRes,
};
use globals::GlobalLayout;
use std::collections::HashSet;
use wgpu::{BlendState, PrimitiveState, TextureFormat};

pub mod globals;
pub mod plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderModel {
    Unlit,
    Lit,
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
    On,
    Off,
}

pub trait MeshPipeline: Sized + Send + Sync + 'static {
    fn new(device: &RenderDevice) -> Self;

    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }
    fn primitive() -> PrimitiveState;
    fn attributes() -> Vec<VertexAttribute>;
    fn shader() -> impl Into<LoadPath>;

    fn bind_group_layout(&self) -> &BindGroupLayout;
}

pub struct MeshPipelineData<M: MeshPipeline> {
    data: M,
}

impl<M: MeshPipeline> MeshPipelineData<M> {
    pub fn new(device: &RenderDevice) -> Self {
        Self {
            data: M::new(device),
        }
    }
}

impl<M: MeshPipeline> std::ops::Deref for MeshPipelineData<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<M: MeshPipeline> std::ops::DerefMut for MeshPipelineData<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<M: MeshPipeline> Resource for MeshPipelineData<M> {}

impl<M: MeshPipeline> RenderResourceExtractor for MeshPipelineData<M> {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self, crate::ExtractError> {
        Ok(Self::new(&arg))
    }
}

pub trait Metadata: Sized + Send + Sync + 'static {
    fn new(device: &RenderDevice) -> Self;
    fn label() -> Option<&'static str>;
    fn model() -> ShaderModel;

    fn bind_group_layout(&self) -> Option<&BindGroupLayout>;
}

pub struct MaterialMetadata<M: Metadata> {
    metadata: M,
}

impl<M: Metadata> MaterialMetadata<M> {
    pub fn new(device: &RenderDevice) -> Self {
        Self {
            metadata: M::new(device),
        }
    }
}

impl<M: Metadata> std::ops::Deref for MaterialMetadata<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl<M: Metadata> std::ops::DerefMut for MaterialMetadata<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

impl<M: Metadata> Resource for MaterialMetadata<M> {}

impl<M: Metadata> RenderResourceExtractor for MaterialMetadata<M> {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self, crate::ExtractError> {
        Ok(Self::new(&arg))
    }
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

pub struct Unlit;

impl Metadata for Unlit {
    fn new(_: &RenderDevice) -> Self {
        Self
    }

    fn label() -> Option<&'static str> {
        None
    }

    fn model() -> ShaderModel {
        ShaderModel::Unlit
    }

    fn bind_group_layout(&self) -> Option<&BindGroupLayout> {
        None
    }
}

pub trait Material: Asset + CreateBindGroup<Data = ()> + 'static {
    type Pipeline: MeshPipeline;
    type Meta: Metadata;

    fn mode() -> BlendMode;
    fn shader() -> impl Into<LoadPath>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType {
    pub surface: Type,
    pub material: Type,
}

impl MaterialType {
    pub fn of<M: Material>() -> Self {
        let surface = Type::of::<M::Pipeline>();
        let material = Type::of::<M>();
        Self { surface, material }
    }
}

#[derive(Debug, Clone)]
pub struct MaterialInstance {
    pub binding: BindGroup,
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub ty: MaterialType,
}

impl RenderAsset for MaterialInstance {
    type Id = AssetId;
}

pub struct MaterialPipeline {
    layout: BindGroupLayout,
    pipeline: RenderPipeline,
    dependencies: HashSet<AssetId>,
}

impl MaterialPipeline {
    pub fn new(layout: BindGroupLayout, pipeline: RenderPipeline, dependency: AssetId) -> Self {
        let mut dependencies = HashSet::new();
        dependencies.insert(dependency);

        Self {
            layout,
            pipeline,
            dependencies,
        }
    }

    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) -> usize {
        self.dependencies.remove(id);

        self.dependencies.len()
    }

    pub fn clear_dependencies(&mut self) {
        self.dependencies.clear();
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }
}

pub struct MaterialPipelineDesc<'a, M: Material> {
    pub format: TextureFormat,
    pub depth_format: Option<TextureFormat>,
    pub global_layout: &'a GlobalLayout,
    pub mesh: &'a M::Pipeline,
    pub metadata: &'a M::Meta,
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
}

pub struct MaterialPipelines {
    pipelines: IndexMap<MaterialType, MaterialPipeline>,
}

impl MaterialPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: IndexMap::new(),
        }
    }

    pub fn get(&self, ty: &MaterialType) -> Option<&MaterialPipeline> {
        self.pipelines.get(ty)
    }

    pub fn has(&self, ty: &MaterialType) -> bool {
        self.pipelines.contains_key(ty)
    }

    pub fn add<M: Material>(
        &mut self,
        device: &RenderDevice,
        desc: MaterialPipelineDesc<M>,
        dependency: AssetId,
    ) -> BindGroupLayout {
        let MaterialPipelineDesc {
            format,
            depth_format,
            global_layout,
            mesh,
            metadata,
            vertex_shader,
            fragment_shader,
        } = desc;

        let ty = MaterialType::of::<M>();
        let material_layout = M::bind_group_layout(device);

        let mut layouts = vec![
            global_layout.inner(),
            mesh.bind_group_layout(),
            &material_layout,
        ];

        let vertex = VertexState {
            shader: vertex_shader,
            entry: vertex_shader.meta().map(|m| m.entry()).unwrap_or("main"),
            buffers: vec![VertexAttribute::into_layout(
                &M::Pipeline::attributes(),
                wgpu::VertexStepMode::Vertex,
            )],
        };

        let fragment = FragmentState {
            shader: fragment_shader,
            entry: fragment_shader.meta().map(|m| m.entry()).unwrap_or("main"),
            targets: vec![Some(wgpu::ColorTargetState {
                format: format,
                blend: Some(M::mode().into()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        if let Some(layout) = metadata.bind_group_layout() {
            layouts.push(layout);
        };

        let desc = RenderPipelineDesc {
            label: M::label(),
            layout: Some(&layouts),
            vertex,
            fragment: Some(fragment),
            primitive: M::Pipeline::primitive(),
            depth_state: match M::Pipeline::depth_write() {
                DepthWrite::On => depth_format.map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                DepthWrite::Off => None,
            },
            multisample: Default::default(),
        };

        let pipeline = RenderPipeline::create(device, desc);
        let pipeline = MaterialPipeline::new(material_layout.clone(), pipeline, dependency);
        self.pipelines.insert(ty, pipeline);

        material_layout
    }

    pub fn add_dependency(&mut self, ty: &MaterialType, id: AssetId) {
        if let Some(pipeline) = self.pipelines.get_mut(ty) {
            pipeline.add_dependency(id);
        }
    }

    pub fn remove_dependency(&mut self, ty: &MaterialType, id: &AssetId) {
        let remove = if let Some(pipeline) = self.pipelines.get_mut(ty) {
            pipeline.remove_dependency(id) == 0
        } else {
            false
        };

        if remove {
            self.pipelines.shift_remove(ty);
        }
    }
}

impl Resource for MaterialPipelines {}
