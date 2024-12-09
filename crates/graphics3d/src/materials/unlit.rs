use asset::Asset;
use graphics::{
    encase::ShaderType,
    resource::{BlendMode, Id, IntoBufferData, Material, MeshPipeline, RenderTexture, Unlit},
    Color, CreateBindGroup,
};

#[derive(serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
#[uniform(0, UnlitColorBufferData)]
pub struct UnlitColor<P: MeshPipeline> {
    pub color: Color,
    _marker: std::marker::PhantomData<P>,
}

impl<P: MeshPipeline> UnlitColor<P> {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P: MeshPipeline> Material for UnlitColor<P> {
    type Pipeline = P;
    type Model = Unlit;

    fn mode() -> graphics::resource::BlendMode {
        BlendMode::Opaque
    }

    fn shader() -> impl Into<asset::io::cache::LoadPath> {
        "graphics3d://assets/unlit-color.wgsl"
    }
}

#[derive(Clone, Copy, ShaderType)]
pub struct UnlitColorBufferData {
    pub color: Color,
}

impl<M: MeshPipeline> IntoBufferData<UnlitColorBufferData> for UnlitColor<M> {
    fn into_buffer_data(&self) -> UnlitColorBufferData {
        UnlitColorBufferData { color: self.color }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
pub struct UnlitTexture<P: MeshPipeline> {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Id<RenderTexture>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: MeshPipeline> UnlitTexture<P> {
    pub fn new(texture: impl Into<Id<RenderTexture>>) -> Self {
        Self {
            texture: texture.into(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P: MeshPipeline> Material for UnlitTexture<P> {
    type Pipeline = P;
    type Model = Unlit;

    fn mode() -> graphics::resource::BlendMode {
        BlendMode::Opaque
    }

    fn shader() -> impl Into<asset::io::cache::LoadPath> {
        "graphics3d://assets/unlit-texture.wgsl"
    }
}
