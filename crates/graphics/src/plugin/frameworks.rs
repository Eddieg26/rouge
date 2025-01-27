use crate::{
    extract::{ExtractError, RenderAssetExtractors, RenderAssets, RenderResourceExtractors},
    plugin::phases::{PostExtract, PostRender, PreRender, Present, Render},
    renderer::{RenderGraph, RenderGraphBuilder},
    resource::{AddRenderTarget, GpuTexture, RenderTarget, RenderTargetEvent, UpdateRenderTarget},
    surface::{RenderSurface, RenderSurfaceError, RenderSurfaceTexture},
    RenderApp, RenderDevice, RenderInstance,
};
use ecs::{
    core::resource::{Res, ResMut},
    event::Events,
    world::{
        action::{BatchEvents, WorldAction, WorldActions},
        World,
    },
};
use game::{ExitGame, Extract, Framework, MainWorld, SubActions};
use pollster::block_on;
use window::{
    events::{WindowCreated, WindowResized},
    Window,
};

pub struct RenderFramework;

impl Framework for RenderFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .add_resource(RenderGraphBuilder::new())
            .observe::<WindowCreated, _>(Self::extract_surface)
            .add_sub_app::<RenderApp>()
            .add_phase::<PreRender>()
            .add_phase::<Render>()
            .add_phase::<PostRender>()
            .add_phase::<Present>()
            .add_resource(RenderSurfaceTexture::default())
            .add_systems(PreRender, Self::set_surface_texture)
            .add_systems(Render, Self::run_render_graph)
            .add_systems(Present, Self::present_surface_texture);
    }
}
impl RenderFramework {
    fn extract_surface(actions: SubActions<RenderApp>) {
        actions.defer::<Extract>(ExtractSurface);
    }

    fn set_surface_texture(
        surface: Res<RenderSurface>,
        mut targets: ResMut<RenderAssets<RenderTarget>>,
        mut surface_texture: ResMut<RenderSurfaceTexture>,
    ) {
        let texture = match surface.texture() {
            Ok(texture) => texture,
            Err(_) => return,
        };

        targets
            .get_mut(&RenderSurface::ID)
            .map(|target| target.color = texture.texture.create_view(&Default::default()));

        surface_texture.set(texture);
    }

    fn run_render_graph(mut graph: Option<ResMut<RenderGraph>>, world: &World) {
        if let Some(graph) = graph.as_mut() {
            graph.run(world);
        }
    }

    fn present_surface_texture(
        mut surface_texture: ResMut<RenderSurfaceTexture>,
        mut textures: ResMut<RenderAssets<GpuTexture>>,
    ) {
        surface_texture.present();
        textures.remove(&RenderSurface::ID.to());
    }
}

pub struct ResizeFramework;

impl Framework for ResizeFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .register_event::<WindowResized>()
            .observe::<WindowResized, _>(Self::extract_resize_events)
            .sub_app_mut::<RenderApp>()
            .register_event::<WindowResized>()
            .observe::<WindowResized, _>(Self::resize_surface)
            .observe::<RenderTargetEvent, _>(Self::resize_render_graph);
    }
}

impl ResizeFramework {
    fn extract_resize_events(events: Res<Events<WindowResized>>, actions: SubActions<RenderApp>) {
        actions.defer::<Extract>(BatchEvents::new(events.iter().copied()));
    }

    fn resize_surface(
        actions: WorldActions,
        events: Res<Events<WindowResized>>,
        device: Res<RenderDevice>,
        mut surface: ResMut<RenderSurface>,
        mut texture: ResMut<RenderSurfaceTexture>,
        mut targets: ResMut<RenderAssets<RenderTarget>>,
    ) {
        if let Some(event) = events.last() {
            texture.destroy();
            surface.resize(&device, event.size.width, event.size.height);

            if let Some(target) = targets.get_mut(&RenderSurface::ID) {
                target.width = event.size.width;
                target.height = event.size.height;
            }

            actions.add(UpdateRenderTarget::new(RenderSurface::ID));
        }
    }

    fn resize_render_graph(
        targets: Res<RenderAssets<RenderTarget>>,
        device: Res<RenderDevice>,
        mut graph: Option<ResMut<RenderGraph>>,
    ) {
        if let Some(graph) = graph.as_mut() {
            let (width, height) = targets.max_size();
            graph.resize(&device, width, height);
        }
    }
}

pub struct ExtractSurface;

impl WorldAction for ExtractSurface {
    fn execute(self, world: &mut World) -> Option<()> {
        let runner = async {
            let window = world.resource::<MainWorld>().non_send_resource::<Window>();
            let instance = RenderInstance::create();

            let mut surface = match RenderSurface::create(&instance, &window).await {
                Ok(surface) => surface,
                Err(error) => return Err(CreateSurfaceError::Surface(error)),
            };

            let device = match RenderDevice::create(surface.adapter()).await {
                Ok(device) => device,
                Err(error) => return Err(CreateSurfaceError::Device(error)),
            };

            surface.configure(&device);

            let size = wgpu::Extent3d {
                width: surface.width(),
                height: surface.height(),
                depth_or_array_layers: 1,
            };

            let color = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Surface Color"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface.format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[surface.format()],
            });

            let depth = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Surface Depth"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface.depth_format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[surface.depth_format()],
            });

            let target = RenderTarget {
                width: surface.width(),
                height: surface.height(),
                color: color.create_view(&Default::default()),
                depth: Some(depth.create_view(&Default::default())),
            };

            world.add_resource(surface);
            world.add_resource(device);

            world
                .actions()
                .add(AddRenderTarget::new(RenderSurface::ID, target));

            Ok(())
        };

        if let Err(error) = block_on(runner) {
            world
                .resource::<MainWorld>()
                .actions()
                .add(ExitGame::failure(error));
        }

        Some(())
    }
}

#[derive(Debug)]
pub enum CreateSurfaceError {
    Surface(RenderSurfaceError),
    Device(wgpu::RequestDeviceError),
}

impl std::fmt::Display for CreateSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(error) => write!(f, "Failed to create surface: {}", error),
            Self::Device(error) => write!(f, "Failed to create device: {}", error),
        }
    }
}

impl std::error::Error for CreateSurfaceError {}

pub struct ExtractFramework;

impl Framework for ExtractFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .add_resource(RenderAssetExtractors::new())
            .add_resource(RenderResourceExtractors::default())
            .register_event::<ExtractError>()
            .sub_app_mut::<RenderApp>()
            .add_sub_phase::<Extract, PostExtract>()
            .register_event::<ExtractError>();
    }
}
