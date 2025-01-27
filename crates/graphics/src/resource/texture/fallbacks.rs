use super::{
    GpuTexture, Sampler, SamplerDesc, Texture1d, Texture2d, Texture2dArray, Texture3d, TextureCube,
    TextureCubeArray, TextureDimension,
};
use crate::{extract::RenderResourceExtractor, RenderDevice};
use ecs::{core::resource::Resource, system::ArgItem};

pub struct Fallbacks {
    pub d1: GpuTexture,
    pub d2_array: GpuTexture,
    pub d2: GpuTexture,
    pub d3: GpuTexture,
    pub cube: GpuTexture,
    pub cube_array: GpuTexture,
    pub sampler: Sampler,
}

impl Fallbacks {
    pub fn new(device: &RenderDevice) -> Self {
        let sampler = Sampler::create(device, &SamplerDesc::default());
        let d1 = GpuTexture::create(device, &Texture1d::default(), sampler.clone());
        let d2 = GpuTexture::create(device, &Texture2d::default(), sampler.clone());
        let d2_array = GpuTexture::create(device, &Texture2dArray::default(), sampler.clone());
        let d3 = GpuTexture::create(device, &Texture3d::default(), sampler.clone());
        let cube = GpuTexture::create(device, &TextureCube::default(), sampler.clone());
        let cube_array = GpuTexture::create(device, &TextureCubeArray::default(), sampler.clone());

        Self {
            d1,
            d2_array,
            d2,
            d3,
            cube,
            cube_array,
            sampler,
        }
    }

    pub fn texture(&self, dimension: TextureDimension) -> &GpuTexture {
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
    type Arg = ();

    fn extract(device: &RenderDevice, _: ArgItem<Self::Arg>) -> Self {
        Self::new(&device)
    }
}
