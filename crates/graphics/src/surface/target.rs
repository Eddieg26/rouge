use crate::{
    core::{RenderAsset, RenderAssetExtractor, RenderAssets, RenderDevice},
    resources::{
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
    system::{
        unlifetime::{ReadRes, WriteRes},
        StaticArg,
    },
    world::action::WorldAction,
};
use spatial::size::Size;

pub struct RenderTarget {
    pub size: Size,
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
    type Arg = StaticArg<
        'static,
        (
            ReadRes<RenderDevice>,
            WriteRes<RenderAssets<RenderTexture>>,
            WriteRes<RenderAssets<Sampler>>,
        ),
    >;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::core::ExtractError> {
        let (device, textures, samplers) = arg.inner_mut();

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
            size: Size::new(source.width(), source.height()),
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
        let (.., textures, samplers) = arg.inner_mut();
        if let Some(target) = assets.remove(&id.into()) {
            textures.remove(&target.color);
            samplers.remove(&target.sampler);
        }
    }
}

impl RenderAssets<RenderTarget> {
    pub fn max_size(&self) -> Size {
        self.iter()
            .map(|(_, target)| target.size)
            .fold(Size::ZERO, |acc, size| acc.max(size))
    }

    pub fn min_size(&self) -> Size {
        self.iter()
            .map(|(_, target)| target.size)
            .fold(Size::MAX, |acc, size| acc.min(size))
    }
}

pub struct ResizeRenderGraph;
impl Event for ResizeRenderGraph {}
impl WorldAction for ResizeRenderGraph {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        Some(world.resource_mut::<Events<Self>>().add(self))
    }
}
