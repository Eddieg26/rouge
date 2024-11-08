use crate::{
    core::{RenderAsset, RenderAssetExtractor, RenderAssets, RenderDevice},
    resources::{
        texture::{
            render::RenderTargetTexture,
            sampler::{Sampler, SamplerDesc},
            RenderTexture, Texture, TextureDesc, TextureFormat,
        },
        Id,
    },
};
use asset::AssetId;
use ecs::system::{
    unlifetime::{ReadRes, WriteRes},
    StaticSystemArg,
};
use spatial::size::Size;

pub struct RenderTarget {
    pub size: Size,
    pub format: TextureFormat,
    pub depth_format: TextureFormat,
    pub color: Id<RenderTexture>,
    pub depth: Id<RenderTexture>,
    pub sampler: Id<Sampler>,
}

impl RenderAsset for RenderTarget {
    type Id = Id<RenderTarget>;
}

impl RenderAssetExtractor for RenderTarget {
    type Source = RenderTargetTexture;
    type Asset = RenderTarget;
    type Arg = StaticSystemArg<
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
        let depth = RenderTexture::from_desc(
            device,
            &TextureDesc {
                label: None,
                width: source.width(),
                height: source.height(),
                depth: source.depth(),
                mipmaps: source.mipmaps(),
                format: source.depth_format(),
                dimension: source.dimension(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                pixels: vec![],
            },
        );
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
        let depth_id = Id::<RenderTexture>::generate();
        let sampler_id = Id::<Sampler>::from(id);

        textures.add(color_id, color);
        textures.add(depth_id, depth);
        samplers.add(sampler_id, sampler);

        Ok(RenderTarget {
            size: Size::new(source.width(), source.height()),
            format: source.format(),
            depth_format: source.depth_format(),
            color: color_id,
            depth: depth_id,
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
            textures.remove(&target.depth);
            samplers.remove(&target.sampler);
        }
    }
}
