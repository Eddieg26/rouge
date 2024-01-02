use crate::ecs::Resource;
use winit::window::Window;

pub struct RenderSurface {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    alpha_mode: wgpu::CompositeAlphaMode,
    width: u32,
    height: u32,
}

impl RenderSurface {
    pub fn new(window: &Window, instance: &wgpu::Instance) -> RenderSurface {
        let surface = unsafe { instance.create_surface(window) }.expect("Failed to create surface");
        let size = window.inner_size();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Failed to create adapter");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = surface_caps.present_modes[0];
        let alpha_mode = surface_caps.alpha_modes[0];

        RenderSurface {
            surface,
            adapter,
            format: surface_format,
            depth_format: wgpu::TextureFormat::Depth32Float,
            present_mode,
            alpha_mode,
            width: size.width,
            height: size.height,
        }
    }

    pub fn inner(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    pub fn present_mode(&self) -> wgpu::PresentMode {
        self.present_mode
    }

    pub fn alpha_mode(&self) -> wgpu::CompositeAlphaMode {
        self.alpha_mode
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn capabilities(&self) -> wgpu::SurfaceCapabilities {
        self.surface.get_capabilities(&self.adapter)
    }

    pub fn default_config(&self, width: u32, height: u32) -> wgpu::SurfaceConfiguration {
        self.surface
            .get_default_config(&self.adapter, width, height)
            .unwrap()
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) {
        let config = self.default_config(size.width, size.height);

        self.width = size.width;
        self.height = size.height;
        self.surface.configure(device, &config);
    }
}

impl Resource for RenderSurface {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
