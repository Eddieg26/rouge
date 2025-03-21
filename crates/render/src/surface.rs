use crate::{app::RenderApp, device::RenderDevice};
use ecs::{
    Res, ResMut, Resource,
    event::Events,
    world::action::{BatchEvents, WorldAction},
};
use game::{ExitGame, Extract, MainWorld, SubActions};
use wgpu::{PresentMode, SurfaceConfiguration, SurfaceTargetUnsafe, rwh::HandleError};
use window::{Window, events::WindowResized};

#[derive(Debug)]
pub enum RenderSurfaceError {
    Create(wgpu::CreateSurfaceError),
    Adapter,
    Handle(HandleError),
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
            Self::Handle(e) => write!(f, "{}", e),
        }
    }
}

impl From<HandleError> for RenderSurfaceError {
    fn from(error: HandleError) -> Self {
        Self::Handle(error)
    }
}

impl std::error::Error for RenderSurfaceError {}

pub struct RenderSurface {
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    depth_format: wgpu::TextureFormat,
}

impl RenderSurface {
    pub const DEFAULT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub async fn new(window: &Window) -> Result<(Self, wgpu::Adapter), RenderSurfaceError> {
        let instance = wgpu::Instance::default();

        let surface = unsafe {
            let target = SurfaceTargetUnsafe::from_window(window.inner())
                .map_err(|e| RenderSurfaceError::from(e))?;

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

        let capabilities = surface.get_capabilities(&adapter);

        let format = *capabilities
            .formats
            .iter()
            .find(|format| **format == Self::DEFAULT_FORMAT)
            .unwrap_or(capabilities.formats.get(0).expect("No supported formats"));

        let depth_format = Self::DEPTH_FORMAT;

        let present_mode = capabilities
            .present_modes
            .iter()
            .find(|mode| **mode == PresentMode::Mailbox)
            .cloned()
            .unwrap_or_default();

        let config = wgpu::SurfaceConfiguration {
            usage: capabilities.usages - wgpu::TextureUsages::STORAGE_BINDING,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 3,
        };

        let surface = Self {
            surface,
            config,
            depth_format,
        };

        Ok((surface, adapter))
    }

    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    pub fn configure(&self, device: &RenderDevice) {
        self.surface.configure(device, &self.config);
    }

    pub fn texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn resize(&mut self, device: &RenderDevice, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }

    pub(crate) fn extract_surface(actions: Res<SubActions<RenderApp>>) {
        actions.defer::<Extract>(ExtractSurface);
    }

    pub(crate) fn extract_resize_events(
        events: Res<Events<WindowResized>>,
        actions: SubActions<RenderApp>,
    ) {
        actions.defer::<Extract>(BatchEvents::new(events.iter().cloned()));
    }

    pub(crate) fn resize_surface(
        events: Res<Events<WindowResized>>,
        device: Res<RenderDevice>,
        mut surface: ResMut<RenderSurface>,
    ) {
        if let Some(event) = events.last() {
            surface.resize(&device, event.width(), event.height());
        }
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
        assert!(self.0.is_none());
        self.0 = Some(texture);
    }

    pub fn present(&mut self) -> Option<()> {
        let texture = self.0.take()?;
        Some(texture.present())
    }

    pub fn destroy(&mut self) {
        std::mem::drop(self.0.take());
    }
}

impl Resource for RenderSurfaceTexture {}

pub struct ExtractSurface;

impl WorldAction for ExtractSurface {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let window = world
            .try_resource::<MainWorld>()?
            .try_non_send_resource::<Window>()?;

        let (surface, adapter) = match pollster::block_on(RenderSurface::new(window)) {
            Ok(surface) => surface,
            Err(error) => {
                world.actions().add(ExitGame::failure(error));
                return None;
            }
        };

        let device = match pollster::block_on(RenderDevice::new(&adapter)) {
            Ok(device) => device,
            Err(error) => {
                world.actions().add(ExitGame::failure(error));
                return None;
            }
        };

        surface.configure(&device);

        world.add_resource(surface);
        world.add_resource(device);

        None
    }
}
