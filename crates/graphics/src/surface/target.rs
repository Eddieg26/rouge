use crate::{
    core::{RenderAsset, RenderDevice},
    surface::RenderSurface,
};
use asset::AssetId;
use spatial::size::Size;

pub struct RenderTargetDesc<'a> {
    pub width: u32,
    pub height: u32,
    pub surface: &'a RenderSurface,
}

pub struct RenderTarget {
    size: Size,
    color: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
}

impl RenderTarget {
    pub fn new(device: &RenderDevice, desc: RenderTargetDesc) -> Self {
        todo!()
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn color(&self) -> Option<&wgpu::TextureView> {
        self.color.as_ref()
    }

    pub fn depth(&self) -> &wgpu::TextureView {
        &self.depth
    }

    pub fn set_color(&mut self, color: Option<wgpu::TextureView>) {
        self.color = color;
    }
}

impl RenderAsset for RenderTarget {
    type Id = AssetId;
}
