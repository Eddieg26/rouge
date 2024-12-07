use asset::{io::cache::LoadPath, Asset};
use ecs::core::resource::Resource;
use graphics::{
    encase::ShaderType,
    resource::{
        BindGroup, BindGroupLayout, BlendMode, Id, IntoBindGroupData, IntoBufferData, Material,
        MeshPipeline, MeshPipelineData, RenderTexture, ShaderModel, Unlit,
    },
    wgpu::PrimitiveState,
    Color, CreateBindGroup, RenderDevice,
};

pub struct TestSurface;

impl MeshPipelineData for TestSurface {
    fn new(device: &RenderDevice) -> Self {
        todo!()
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        todo!()
    }
}

impl MeshPipeline for TestSurface {
    type Data = Self;

    fn primitive() -> PrimitiveState {
        PrimitiveState::default()
    }

    fn shader() -> impl Into<LoadPath> {
        ""
    }

    fn attributes() -> Vec<graphics::resource::VertexAttribute> {
        vec![]
    }
}

impl Resource for TestSurface {}

#[derive(serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
pub struct Standard<S: MeshPipeline> {
    albedo_color: Color,
    other_color: Color,
    #[texture(1)]
    #[sampler(1)]
    albedo_texture: Option<Id<RenderTexture>>,
    _marker: std::marker::PhantomData<S>,
}

impl<S: MeshPipeline> Material for Standard<S> {
    type Pipeline = S;
    type Meta = Unlit;

    fn mode() -> BlendMode {
        BlendMode::Opaque
    }

    fn shader() -> impl Into<LoadPath> {
        ""
    }
}

impl<S: MeshPipeline> IntoBufferData<StandardBufferData> for Standard<S> {
    fn into_buffer_data(&self) -> StandardBufferData {
        StandardBufferData {
            albedo_color: self.albedo_color,
            other_color: self.other_color,
        }
    }
}

impl<S: MeshPipeline> IntoBindGroupData<StandardBufferData> for Standard<S> {
    fn into_bind_group_data(&self) -> StandardBufferData {
        StandardBufferData {
            albedo_color: self.albedo_color,
            other_color: self.other_color,
        }
    }
}

#[derive(ShaderType)]
pub struct StandardBufferData {
    albedo_color: Color,
    other_color: Color,
}
