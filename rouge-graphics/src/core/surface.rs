use rouge_core::primitives::Size;
use rouge_ecs::{macros::Resource, world::resource::Resource};

#[derive(Resource)]
pub struct RenderSurface {
    surface: Option<wgpu::Surface>,
    adapter: wgpu::Adapter,
    format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    alpha_mode: wgpu::CompositeAlphaMode,
    size: Size,
}

impl RenderSurface {
    pub fn new<W>(instance: &wgpu::Instance, window_handle: &W, size: Size) -> Self
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let surface = unsafe { instance.create_surface(window_handle) }.ok();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            ..Default::default()
        }))
        .expect("Failed to find an appropriate adapter");

        let (format, present_mode, alpha_mode) = match &surface {
            Some(surface) => {
                let surface_caps = surface.get_capabilities(&adapter);
                let surface_format = surface_caps
                    .formats
                    .iter()
                    .copied()
                    .find(|f| f.is_srgb())
                    .unwrap_or(surface_caps.formats[0]);

                let present_mode = surface_caps.present_modes[0];
                let alpha_mode = surface_caps.alpha_modes[0];

                (surface_format, present_mode, alpha_mode)
            }
            None => (
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::PresentMode::Fifo,
                wgpu::CompositeAlphaMode::Opaque,
            ),
        };

        Self {
            surface,
            adapter,
            format,
            depth_format: wgpu::TextureFormat::Depth32Float,
            present_mode,
            alpha_mode,
            size,
        }
    }

    pub fn inner(&self) -> Option<&wgpu::Surface> {
        self.surface.as_ref()
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

    pub fn capabilities(&self) -> Option<wgpu::SurfaceCapabilities> {
        self.surface
            .as_ref()
            .map(|surface| surface.get_capabilities(&self.adapter))
    }

    pub fn size(&self) -> Size {
        self.size
    }

    fn default_config_inner(
        present_mode: wgpu::PresentMode,
        alpha_mode: wgpu::CompositeAlphaMode,
        format: wgpu::TextureFormat,
        size: Size,
    ) -> wgpu::SurfaceConfiguration {
        wgpu::SurfaceConfiguration {
            alpha_mode,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: vec![],
        }
    }

    pub fn default_config(&self, size: Size) -> wgpu::SurfaceConfiguration {
        match self.surface.as_ref() {
            Some(surface) => surface
                .get_default_config(&self.adapter, size.width, size.height)
                .unwrap_or(Self::default_config_inner(
                    self.present_mode,
                    self.alpha_mode,
                    self.format,
                    size,
                )),
            None => {
                Self::default_config_inner(self.present_mode, self.alpha_mode, self.format, size)
            }
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) {
        self.surface.as_ref().map(|surface| {
            let config = self.default_config(size);
            surface.configure(device, &config);
        });
        self.size = size;
    }
}
