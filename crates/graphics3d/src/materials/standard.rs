use asset::{io::cache::LoadPath, Asset};
use graphics::{
    encase::ShaderType,
    resource::{
        BlendMode, Id, IntoBindGroupData, IntoBufferData, Material, MeshPipeline, RenderTexture,
        Unlit,
    },
    Color, CreateBindGroup,
};

#[derive(serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
#[uniform(0, StandardBufferData)]
pub struct Standard<S: MeshPipeline> {
    albedo_color: Color,
    other_color: Color,
    #[texture(1)]
    #[sampler(2)]
    albedo_texture: Option<Id<RenderTexture>>,
    _marker: std::marker::PhantomData<S>,
}

impl<S: MeshPipeline> Material for Standard<S> {
    type Pipeline = S;
    type Model = Unlit;

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