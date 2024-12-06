use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use crate::{
    core::{RenderAsset, RenderAssetExtractor, RenderAssets, RenderDevice},
    resource::{
        texture::{
            sampler::{Sampler, SamplerDesc},
            RenderTexture,
        },
        Id,
    },
};
use asset::{Asset, AssetId};
use ecs::{
    event::{Event, Events},
    system::unlifetime::{ReadRes, WriteRes},
    world::action::WorldAction,
};
use std::ops::Range;
use wgpu::TextureUsages;

pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub color: Id<RenderTexture>,
    pub sampler: Id<Sampler>,
}

impl RenderAsset for RenderTarget {
    type Id = Id<RenderTarget>;
}

impl RenderAssetExtractor for RenderTarget {
    type Source = RenderTargetTexture;
    type Asset = RenderTarget;
    type Arg = (
        ReadRes<RenderDevice>,
        WriteRes<RenderAssets<RenderTexture>>,
        WriteRes<RenderAssets<Sampler>>,
    );

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::core::ExtractError> {
        let (device, textures, samplers) = arg;

        let color = RenderTexture::create(device, source);
        let sampler = Sampler::create(
            device,
            &SamplerDesc {
                label: None,
                wrap_mode: source.wrap_mode(),
                filter_mode: source.filter_mode(),
                ..Default::default()
            },
        );

        let color_id = Id::<RenderTexture>::from(id);
        let sampler_id = Id::<Sampler>::from(id);

        textures.add(color_id, color);
        samplers.add(sampler_id, sampler);

        Ok(RenderTarget {
            width: source.width(),
            height: source.height(),
            format: source.format(),
            color: color_id,
            sampler: sampler_id,
        })
    }

    fn remove(
        id: &AssetId,
        assets: &mut crate::core::RenderAssets<Self::Asset>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (.., textures, samplers) = arg;
        if let Some(target) = assets.remove(&id.into()) {
            textures.remove(&target.color);
            samplers.remove(&target.sampler);
        }
    }
}

impl RenderAssets<RenderTarget> {
    pub fn max_size(&self) -> (u32, u32) {
        self.iter()
            .map(|(_, target)| (target.width, target.height))
            .fold((0, 0), |acc, size| acc.max(size))
    }

    pub fn min_size(&self) -> (u32, u32) {
        self.iter()
            .map(|(_, target)| (target.width, target.height))
            .fold((0, 0), |acc, size| acc.min(size))
    }
}

pub struct ResizeRenderGraph;
impl Event for ResizeRenderGraph {}
impl WorldAction for ResizeRenderGraph {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        Some(world.resource_mut::<Events<Self>>().add(self))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Asset)]
pub struct RenderTargetTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    pixels: Vec<u8>,
    faces: [super::TextureFace; 1],
}

impl RenderTargetTexture {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
    ) -> Self {
        let faces = [super::TextureFace::new(0, 0)];

        Self {
            wrap_mode,
            height,
            format,
            filter_mode,
            width,
            pixels: Vec::new(),
            faces,
        }
    }
}

impl Texture for RenderTargetTexture {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        1
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D2
    }

    fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    fn mipmaps(&self) -> bool {
        false
    }

    fn usage(&self) -> TextureUsages {
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING
    }

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}
