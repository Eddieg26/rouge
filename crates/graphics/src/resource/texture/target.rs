use crate::{
    core::{RenderAsset, RenderAssetExtractor, RenderAssets, RenderDevice},
    resource::{
        texture::{
            render::RenderTargetTexture,
            sampler::{Sampler, SamplerDesc},
            RenderTexture, Texture, TextureFormat,
        },
        Id,
    },
};
use asset::AssetId;
use ecs::{
    event::{Event, Events},
    system::unlifetime::{ReadRes, WriteRes},
    world::action::WorldAction,
};

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
