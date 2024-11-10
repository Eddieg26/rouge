use crate::resources::{texture::TextureFormat, Id};
use ecs::core::resource::Resource;
use spatial::size::Size;
use target::RenderTarget;
use wgpu::{
    rwh::{HandleError, HasDisplayHandle, HasWindowHandle},
    SurfaceTargetUnsafe,
};
use window::Window;

pub mod target;

#[derive(Debug)]
pub enum RenderSurfaceError {
    Create(wgpu::CreateSurfaceError),
    Adapter,
    DisplayHandle(HandleError),
    WindowHandle(HandleError),
}

impl From<wgpu::CreateSurfaceError> for RenderSurfaceError {
    fn from(error: wgpu::CreateSurfaceError) -> Self {
        Self::Create(error)
    }
}

impl std::fmt::Display for RenderSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create(e) => write!(f, "Failed to create surface: {}", e),
            Self::Adapter => write!(f, "Failed to request adapter"),
            Self::DisplayHandle(e) => write!(f, "{}", e),
            Self::WindowHandle(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RenderSurfaceError {}

pub struct RenderSurface {
    inner: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    config: wgpu::SurfaceConfiguration,
    format: TextureFormat,
    depth_format: TextureFormat,
}

impl RenderSurface {
    pub const ID: Id<RenderTarget> = Id::new(0);

    pub async fn create(
        instance: &wgpu::Instance,
        window: &Window,
    ) -> Result<Self, RenderSurfaceError> {
        let surface = unsafe {
            let window_handle = window
                .inner()
                .window_handle()
                .map_err(|e| RenderSurfaceError::WindowHandle(e))?;

            let display_handle = window
                .inner()
                .display_handle()
                .map_err(|e| RenderSurfaceError::DisplayHandle(e))?;

            let target = SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: display_handle.into(),
                raw_window_handle: window_handle.into(),
            };
            instance
                .create_surface_unsafe(target)
                .map_err(|e| RenderSurfaceError::from(e))?
        };

        let size = window.size();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or(RenderSurfaceError::Adapter)?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let depth_format = TextureFormat::Depth32Float;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 3,
        };

        Ok(Self {
            inner: surface,
            adapter,
            config,
            format: TextureFormat::from(surface_format),
            depth_format,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.inner.configure(device, &self.config);
    }

    pub fn configure(&mut self, device: &wgpu::Device) {
        self.inner.configure(device, &self.config);
    }

    pub fn inner(&self) -> &wgpu::Surface {
        &self.inner
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn depth_format(&self) -> TextureFormat {
        self.depth_format
    }

    pub fn texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.inner.get_current_texture()
    }
}

impl Resource for RenderSurface {}

#[derive(Debug, Default)]
pub struct RenderSurfaceTexture(Option<wgpu::SurfaceTexture>);

impl RenderSurfaceTexture {
    pub fn new(texture: wgpu::SurfaceTexture) -> Self {
        Self(Some(texture))
    }

    pub fn get(&self) -> Option<&wgpu::SurfaceTexture> {
        self.0.as_ref()
    }

    pub fn set(&mut self, texture: wgpu::SurfaceTexture) {
        self.0 = Some(texture);
    }

    pub fn present(&mut self) -> Option<()> {
        let texture = self.0.take()?;
        Some(texture.present())
    }
}

impl Resource for RenderSurfaceTexture {}
