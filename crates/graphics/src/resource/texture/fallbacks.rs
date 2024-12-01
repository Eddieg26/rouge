use super::{
    RenderTexture, Sampler, SamplerDesc, Texture1d, Texture2d, Texture2dArray, Texture3d,
    TextureCube, TextureCubeArray, TextureDimension,
};
use crate::{RenderDevice, RenderResourceExtractor};
use ecs::{core::resource::Resource, system::unlifetime::ReadRes};

pub struct Fallbacks {
    pub d1: RenderTexture,
    pub d2_array: RenderTexture,
    pub d2: RenderTexture,
    pub d3: RenderTexture,
    pub cube: RenderTexture,
    pub cube_array: RenderTexture,
    pub sampler: Sampler,
}

impl Fallbacks {
    pub fn new(device: &RenderDevice) -> Self {
        Self {
            d1: RenderTexture::create(&device, &Texture1d::default()),
            d2: RenderTexture::create(&device, &Texture2d::default()),
            d2_array: RenderTexture::create(&device, &Texture2dArray::default()),
            d3: RenderTexture::create(&device, &Texture3d::default()),
            cube: RenderTexture::create(&device, &TextureCube::default()),
            cube_array: RenderTexture::create(&device, &TextureCubeArray::default()),
            sampler: Sampler::create(&device, &SamplerDesc::default()),
        }
    }

    pub fn texture(&self, dimension: TextureDimension) -> &RenderTexture {
        match dimension {
            TextureDimension::D1 => &self.d1,
            TextureDimension::D2 => &self.d2,
            TextureDimension::D3 => &self.d3,
            TextureDimension::Cube => &self.cube,
            TextureDimension::D2Array => &self.d2_array,
            TextureDimension::CubeArray => &self.cube_array,
        }
    }
}

impl Resource for Fallbacks {}

impl RenderResourceExtractor for Fallbacks {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(device: ecs::system::ArgItem<Self::Arg>) -> Result<Self, crate::ExtractError> {
        Ok(Self::new(&device))
    }
}
