use super::{FilterMode, GpuTexture, Sampler, Texture, WrapMode};
use crate::{
    device::RenderDevice,
    resources::{
        Id,
        extract::{ExtractError, RenderAsset, RenderAssetExtractor, RenderAssets},
    },
};
use asset::Asset;
use ecs::{
    event::{Event, Events},
    system::unlifetime::ReadRes,
    world::action::{WorldAction, WorldActions},
};
use wgpu::TextureFormat;

pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub color: wgpu::TextureView,
    pub depth: Option<wgpu::TextureView>,
}

impl RenderAsset for RenderTarget {}

#[derive(serde::Serialize, serde::Deserialize, Asset)]
pub struct RenderTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    depth_format: Option<TextureFormat>,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    faces: [super::TextureFace; 1],
}

impl Texture for RenderTexture {
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

    fn dimension(&self) -> super::TextureDimension {
        super::TextureDimension::D2
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

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
    }

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, _: std::ops::Range<usize>) -> &[u8] {
        &[]
    }
}

impl RenderAssetExtractor for RenderTexture {
    type RenderAsset = GpuTexture;
    type Arg = (ReadRes<RenderDevice>, WorldActions);

    fn extract(
        id: &asset::AssetId,
        texture: &mut Self,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::RenderAsset, ExtractError> {
        let (device, actions) = arg;

        let size = wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        };

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture.format,
            usage: texture.usage(),
            view_formats: &[texture.format],
        });

        let depth = texture.depth_format.map(|format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: texture.usage(),
                view_formats: &[format],
            })
        });

        let target = RenderTarget {
            width: size.width,
            height: size.height,
            color: color.create_view(&Default::default()),
            depth: depth.map(|depth| depth.create_view(&Default::default())),
        };

        actions.add(AddRenderTarget::new(id, target));

        let sampler = Sampler::from_texture(device, texture);

        Ok(GpuTexture::create(device, texture, sampler))
    }

    fn update(
        id: &asset::AssetId,
        _: &mut Self,
        _: &mut Self::RenderAsset,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        let (_, actions) = arg;

        actions.add(UpdateRenderTarget::new(id));

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderTargetEvent {
    Add { id: Id<RenderTarget> },
    Remove { id: Id<RenderTarget> },
    Update { id: Id<RenderTarget> },
}

impl Event for RenderTargetEvent {}

pub struct AddRenderTarget {
    id: Id<RenderTarget>,
    target: RenderTarget,
}

impl AddRenderTarget {
    pub fn new(id: impl Into<Id<RenderTarget>>, target: RenderTarget) -> Self {
        Self {
            id: id.into(),
            target,
        }
    }
}

impl WorldAction for AddRenderTarget {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<RenderAssets<RenderTarget>>()
            .add(self.id, self.target);

        world
            .resource_mut::<Events<RenderTargetEvent>>()
            .add(RenderTargetEvent::Add { id: self.id });

        Some(())
    }
}

pub struct RemoveRenderTarget {
    id: Id<RenderTarget>,
}

impl RemoveRenderTarget {
    pub fn new(id: impl Into<Id<RenderTarget>>) -> Self {
        Self { id: id.into() }
    }
}

impl WorldAction for RemoveRenderTarget {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<RenderAssets<RenderTarget>>()
            .remove(&self.id)?;
        world
            .resource_mut::<Events<RenderTargetEvent>>()
            .add(RenderTargetEvent::Remove { id: self.id });

        Some(())
    }
}

pub struct UpdateRenderTarget {
    id: Id<RenderTarget>,
}

impl UpdateRenderTarget {
    pub fn new(id: impl Into<Id<RenderTarget>>) -> Self {
        Self { id: id.into() }
    }
}

impl WorldAction for UpdateRenderTarget {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<RenderTargetEvent>>()
            .add(RenderTargetEvent::Update { id: self.id });

        Some(())
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
