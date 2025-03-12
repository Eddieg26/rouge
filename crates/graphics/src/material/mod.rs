use crate::{
    extract::{
        ExtractError, ExtractPipeline, PipelineExtractor, PipelineShaders, RenderAsset,
        RenderAssetExtractor, RenderAssets, RenderResourceExtractor,
    },
    resource::{
        BindGroup, BindGroupLayout, BufferData, CreateBindGroup, DynamicOffset, FragmentState,
        MeshAttributeKind, RenderBufferArray, RenderPipeline, RenderPipelineDesc, Shader,
        UniformBufferArray, VertexBufferLayout, VertexState,
    },
    surface::RenderSurface,
    RenderDevice, Viewport,
};
use asset::{asset::Asset, database::AssetDatabase, io::cache::LoadPath, AssetId, AssetRef};
use ecs::{
    core::resource::Resource,
    system::{unlifetime::ReadRes, ArgItem, SystemArg},
    world::{
        action::WorldActions,
        builtin::actions::{AddResource, RemoveResource},
    },
    Entity,
};
use game::SMain;
use std::{marker::PhantomData, num::NonZeroU32};
use wgpu::{BlendState, PrimitiveState, TextureFormat};

pub mod draw;
pub mod plugin;
pub mod v2;

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

pub trait ModelUniform: Copy + BufferData + Send + 'static {}

pub trait View: Send + Sized + 'static {
    type Uniform: ModelUniform;
    type ExtractArg: SystemArg;

    fn extract(arg: ArgItem<Self::ExtractArg>) -> ExtractedView<Self>;
}

pub struct ExtractedView<V: View> {
    pub entity: Entity,
    pub data: V::Uniform,
    pub viewport: Viewport,
}

impl<V: View> Clone for ExtractedView<V> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            data: self.data,
            viewport: self.viewport.clone(),
        }
    }
}

pub struct RenderView<V: View> {
    pub entity: Entity,
    pub data: V::Uniform,
    pub viewport: Viewport,
    pub dynamic_offset: DynamicOffset<V::Uniform>,
}

impl<V: View> Clone for RenderView<V> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            data: self.data,
            viewport: self.viewport.clone(),
            dynamic_offset: self.dynamic_offset,
        }
    }
}

impl<V: View> RenderAsset for RenderView<V> {
    type Id = Entity;
}

pub struct ViewBinding<V: View> {
    layout: BindGroupLayout,
    binding: BindGroup,
    buffer: UniformBufferArray<V::Uniform>,
}

impl<V: View> Resource for ViewBinding<V> {}
impl<V: View> RenderResourceExtractor for ViewBinding<V> {
    type Arg = ();

    fn extract(device: &crate::RenderDevice, arg: ecs::system::ArgItem<Self::Arg>) -> Self {
        todo!()
    }
}

pub struct MeshBinding<M: MeshRenderer> {
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
    pub buffer: RenderBufferArray<M::Mesh>,
}

impl<M: MeshRenderer> Resource for MeshBinding<M> {}
impl<M: MeshRenderer> RenderResourceExtractor for MeshBinding<M> {
    type Arg = ();

    fn extract(device: &crate::RenderDevice, arg: ecs::system::ArgItem<Self::Arg>) -> Self {
        todo!()
    }
}

pub trait MeshRenderer: Send + Sync + 'static {
    type View: View;
    type Mesh: ModelUniform;

    const INSTANCES: Option<NonZeroU32> = None;

    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }

    fn primitive() -> PrimitiveState;
    fn vertex_attributes() -> Vec<VertexAttribute>;
    fn instance_attributes() -> Vec<VertexAttribute> {
        vec![]
    }
    fn shader() -> impl Into<LoadPath>;
}

pub trait LightBinding: RenderResourceExtractor + Send + Sync + 'static {
    fn bind_group(&self) -> Option<&BindGroup>;
    fn bind_group_layout(&self) -> Option<&BindGroupLayout>;
}

pub trait Material: Asset + CreateBindGroup + 'static {
    type Renderer: MeshRenderer;
    type Lighting: LightBinding;

    fn mode() -> BlendMode;
    fn shader() -> impl Into<LoadPath>;
}

pub type MaterialView<M> = ViewBinding<<<M as Material>::Renderer as MeshRenderer>::View>;
pub type MaterialRenderView<M> = RenderView<<<M as Material>::Renderer as MeshRenderer>::View>;
pub type MaterialRenderViews<M> =
    RenderAssets<RenderView<<<M as Material>::Renderer as MeshRenderer>::View>>;
pub type MaterialMesh<M> = MeshBinding<<M as Material>::Renderer>;
pub type MaterialMeshData<M> = <<M as Material>::Renderer as MeshRenderer>::Mesh;
pub type MaterialLight<M> = <M as Material>::Lighting;

#[derive(Debug, Clone)]
pub struct MaterialInstance<M: Material>(BindGroup<M::Data>);
impl<M: Material> MaterialInstance<M> {
    pub fn bind_group(&self) -> &BindGroup<M::Data> {
        &self.0
    }
}

impl<M: Material> std::ops::Deref for MaterialInstance<M> {
    type Target = BindGroup<M::Data>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M: Material> RenderAsset for MaterialInstance<M> {
    type Id = AssetRef<M>;
}

impl<M: Material> RenderAssetExtractor for M {
    type Target = MaterialInstance<M>;
    type Arg = (
        ReadRes<RenderDevice>,
        Option<ReadRes<MaterialLayout<M>>>,
        Option<ReadRes<MaterialPipeline<M>>>,
        SMain<WorldActions>,
        M::Arg,
    );

    fn extract(
        _: &AssetId,
        material: &mut Self,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Target, ExtractError> {
        let (device, layout, pipeline, actions, material_arg) = arg;

        let layout = match layout.as_ref() {
            Some(layout) => layout.value().clone(),
            None => {
                let layout = MaterialLayout::<M>::new(&device);
                actions.add(AddResource::new(layout.clone()));
                layout
            }
        };

        let binding =
            match material.create_bind_group(device, &layout.bind_group_layout(), material_arg) {
                Ok(binding) => binding,
                Err(error) => return Err(error.into()),
            };

        if pipeline.is_none() {
            actions.add(ExtractPipeline::<MaterialPipeline<M>>::new());
        }

        Ok(MaterialInstance(binding))
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>, arg: &mut ArgItem<Self::Arg>) {
        if assets.remove(&AssetRef::<Self>::from(id)).is_none() || !assets.is_empty() {
            return;
        }

        let (_, _, _, actions, _) = arg;

        actions.add(RemoveResource::<MaterialLayout<M>>::new());
        actions.add(RemoveResource::<MaterialPipeline<M>>::new());
    }
}

pub struct MaterialLayout<M: Material>(BindGroupLayout, PhantomData<M>);
impl<M: Material> MaterialLayout<M> {
    pub fn new(device: &RenderDevice) -> Self {
        Self(M::create_bind_group_layout(device), PhantomData)
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.0
    }
}

impl<M: Material> std::ops::Deref for MaterialLayout<M> {
    type Target = BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M: Material> Clone for MaterialLayout<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<M: Material> Resource for MaterialLayout<M> {}

pub struct MaterialPipelineDesc<'a, M: Material> {
    pub format: TextureFormat,
    pub depth_format: Option<TextureFormat>,
    pub view: &'a MaterialView<M>,
    pub mesh: &'a MaterialMesh<M>,
    pub light: &'a MaterialLight<M>,
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
}

pub struct MaterialPipeline<M: Material>(RenderPipeline, PhantomData<M>);

impl<M: Material> MaterialPipeline<M> {
    pub fn new(
        device: &RenderDevice,
        layout: &MaterialLayout<M>,
        desc: MaterialPipelineDesc<M>,
    ) -> Self {
        let MaterialPipelineDesc {
            format,
            depth_format,
            view,
            mesh,
            light,
            vertex_shader,
            fragment_shader,
        } = desc;

        let mut layouts = vec![&view.layout, &mesh.layout, layout.bind_group_layout()];

        layouts.push(layout.bind_group_layout());

        let mut vertex_buffer_layouts = vec![VertexAttribute::into_layout(
            &M::Renderer::vertex_attributes(),
            wgpu::VertexStepMode::Vertex,
        )];

        let instance_attributes = M::Renderer::instance_attributes();
        if instance_attributes.len() > 0 {
            vertex_buffer_layouts.push(VertexAttribute::into_layout(
                &instance_attributes,
                wgpu::VertexStepMode::Instance,
            ));
        }

        let vertex = VertexState {
            shader: vertex_shader,
            entry: vertex_shader.meta().map(|m| m.entry()).unwrap_or("main"),
            buffers: vertex_buffer_layouts,
            instances: M::Renderer::INSTANCES,
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

        if let Some(layout) = light.bind_group_layout().as_ref() {
            layouts.push(layout);
        };

        let desc = RenderPipelineDesc {
            label: M::label(),
            layout: Some(&layouts),
            vertex,
            fragment: Some(fragment),
            primitive: M::Renderer::primitive(),
            depth_state: match M::Renderer::depth_write() {
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

        Self(RenderPipeline::create(device, desc), PhantomData)
    }
}

impl<M: Material> std::ops::Deref for MaterialPipeline<M> {
    type Target = RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M: Material> Resource for MaterialPipeline<M> {}

impl<M: Material> PipelineExtractor for MaterialPipeline<M> {
    type Arg = (
        ReadRes<RenderSurface>,
        ReadRes<MaterialLayout<M>>,
        ReadRes<MaterialView<M>>,
        ReadRes<MaterialMesh<M>>,
        ReadRes<MaterialLight<M>>,
        SMain<ReadRes<AssetDatabase>>,
    );

    fn shaders() -> PipelineShaders {
        PipelineShaders::Render {
            vertex: M::Renderer::shader().into(),
            fragment: M::shader().into(),
        }
    }

    fn extract(
        device: &RenderDevice,
        shaders: &RenderAssets<Shader>,
        arg: ArgItem<Self::Arg>,
    ) -> Self {
        let (surface, layout, view, mesh, light, database) = arg;

        let vertex_shader = match M::Renderer::shader().into() {
            LoadPath::Id(id) => shaders.get(&id.into()).unwrap(),
            LoadPath::Path(path) => {
                let library = database.library().read_blocking();
                let id = library.get_id(&path).unwrap();
                shaders.get(&id.into()).unwrap()
            }
        };

        let fragment_shader = match M::shader().into() {
            LoadPath::Id(id) => shaders.get(&id.into()).unwrap(),
            LoadPath::Path(path) => {
                let library = database.library().read_blocking();
                let id = library.get_id(&path).unwrap();
                shaders.get(&id.into()).unwrap()
            }
        };

        let desc = MaterialPipelineDesc {
            format: surface.format(),
            depth_format: Some(surface.depth_format()),
            view: view.value(),
            mesh: mesh.value(),
            light: light.value(),
            vertex_shader,
            fragment_shader,
        };

        Self::new(device, &layout, desc)
    }
}
