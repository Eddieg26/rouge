use super::phases::{PostExtract, PostRender};
use crate::plugin::phases::{PreRender, Present, Render};
use crate::renderer::RenderGraphBuilder;
use crate::resource::extract::PipelineExtractors;
use crate::resource::{
    RenderTarget, RenderTargetTexture, ResizeRenderGraph, Sampler, SamplerDesc, ShaderSource,
};
use crate::surface::RenderSurfaceError;
use crate::{
    renderer::RenderGraph,
    resource::RenderTexture,
    surface::{RenderSurface, RenderSurfaceTexture},
    RenderAssets,
};
use crate::{
    ExtractError, RenderAssetExtractors, RenderDevice, RenderInstance, RenderResourceExtractors,
};
use asset::database::events::AssetEvent;
use asset::database::AssetDatabase;
use asset::io::cache::LoadPath;
use ecs::core::resource::NonSend;
use ecs::event::{Event, Events};
use ecs::world::action::{BatchEvents, WorldAction, WorldActions};
use ecs::{
    core::resource::{Res, ResMut},
    world::World,
};
use game::{AppTag, ExitGame, Extract, Framework, Main, SubActions};
use pollster::block_on;
use window::events::{WindowCreated, WindowResized};
use window::Window;

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}

pub struct RenderFramework;

impl Framework for RenderFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .add_resource(RenderGraphBuilder::new())
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
    fn set_surface_texture(
        surface: Res<RenderSurface>,
        mut textures: ResMut<RenderAssets<RenderTexture>>,
        mut surface_texture: ResMut<RenderSurfaceTexture>,
    ) {
        let surface = match surface.texture() {
            Ok(texture) => texture,
            Err(_) => return,
        };

        let texture = RenderTexture::new(None, surface.texture.create_view(&Default::default()));

        textures.add(RenderSurface::ID.to(), texture);

        surface_texture.set(surface);
    }

    fn run_render_graph(mut graph: Option<ResMut<RenderGraph>>, world: &World) {
        if let Some(graph) = graph.as_mut() {
            graph.run(world);
        }
    }

    fn present_surface_texture(
        mut surface_texture: ResMut<RenderSurfaceTexture>,
        mut textures: ResMut<RenderAssets<RenderTexture>>,
    ) {
        surface_texture.present();
        textures.remove(&RenderSurface::ID.to());
    }
}

pub struct ResizeFramework;

impl Framework for ResizeFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .observe::<WindowResized, _>(Self::extract_resize_events)
            .observe::<AssetEvent<RenderTargetTexture>, _>(Self::on_update_render_targets)
            .sub_app_mut::<RenderApp>()
            .register_event::<WindowResized>()
            .register_event::<ResizeRenderGraph>()
            .observe::<WindowResized, _>(Self::on_window_resized)
            .observe::<ResizeRenderGraph, _>(Self::on_resize_render_graph);
    }
}

impl ResizeFramework {
    fn extract_resize_events(events: Res<Events<WindowResized>>, actions: SubActions<RenderApp>) {
        actions.defer::<Extract>(BatchEvents::new(events.iter().copied()));
    }

    fn on_window_resized(
        events: Res<Events<WindowResized>>,
        device: Res<RenderDevice>,
        mut surface: ResMut<RenderSurface>,
        mut texture: ResMut<RenderSurfaceTexture>,
        mut targets: ResMut<RenderAssets<RenderTarget>>,
        mut updates: ResMut<Events<ResizeRenderGraph>>,
    ) {
        if let Some(event) = events.last() {
            texture.destroy();
            surface.resize(&device, event.size.width, event.size.height);

            if let Some(target) = targets.get_mut(&RenderSurface::ID) {
                target.width = event.size.width;
                target.height = event.size.height;
                updates.add(ResizeRenderGraph);
            }
        }
    }

    fn on_resize_render_graph(
        targets: Res<RenderAssets<RenderTarget>>,
        device: Res<RenderDevice>,
        mut graph: Option<ResMut<RenderGraph>>,
    ) {
        let (width, height) = targets.max_size();
        if let Some(graph) = graph.as_mut() {
            graph.resize(&device, width, height);
        }
    }

    fn on_update_render_targets(
        events: Res<Events<AssetEvent<RenderTargetTexture>>>,
        actions: SubActions<RenderApp>,
    ) {
        if events
            .iter()
            .any(|event| !matches!(event, AssetEvent::Imported { .. }))
        {
            actions.add(ResizeRenderGraph);
        }
    }
}

pub struct CreateSurfaceFramework;

impl Framework for CreateSurfaceFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .observe::<WindowCreated, _>(Self::create_render_surface)
            .sub_app_mut::<RenderApp>()
            .register_event::<SurfaceCreated>();
    }
}

impl CreateSurfaceFramework {
    fn create_render_surface(
        window: NonSend<Window>,
        actions: &WorldActions,
        render_actions: SubActions<RenderApp>,
    ) {
        let instance = RenderInstance::create();
        let actions = actions.clone();
        let runner = async {
            let mut surface = match RenderSurface::create(&instance, &window).await {
                Ok(surface) => surface,
                Err(error) => return Err(CreateSurfaceError::Surface(error)),
            };

            let device = match RenderDevice::create(surface.adapter()).await {
                Ok(device) => device,
                Err(error) => return Err(CreateSurfaceError::Device(error)),
            };

            surface.configure(&device);

            render_actions.add(AddRenderSurface { surface, device });

            Ok(())
        };

        if let Err(error) = block_on(runner) {
            actions.add(ExitGame::failure(error));
        }
    }
}

pub struct SurfaceCreated;
impl Event for SurfaceCreated {}

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

pub struct AddRenderSurface {
    pub surface: RenderSurface,
    pub device: RenderDevice,
}

impl WorldAction for AddRenderSurface {
    fn execute(self, world: &mut World) -> Option<()> {
        let world = unsafe { world.cell() };
        let surface = self.surface;
        let device = self.device;

        let target = RenderTarget {
            width: surface.width(),
            height: surface.height(),
            format: surface.format(),
            color: RenderSurface::ID.to(),
            sampler: RenderSurface::ID.to(),
        };

        let sampler = Sampler::create(&device, &SamplerDesc::default());

        world.get_mut().add_resource(surface);
        world.get_mut().add_resource(device);

        world
            .resource_mut::<RenderAssets<Sampler>>()
            .add(target.sampler, sampler);

        world
            .resource_mut::<RenderAssets<RenderTarget>>()
            .add(RenderSurface::ID, target);

        world
            .resource_mut::<Events<SurfaceCreated>>()
            .add(SurfaceCreated);

        Some(())
    }
}

pub struct ExtractFramework;

impl Framework for ExtractFramework {
    fn apply(&self, context: &mut game::FrameworkContext) {
        context
            .add_resource(RenderAssetExtractors::new())
            .add_resource(RenderResourceExtractors::new())
            .add_resource(PipelineExtractors::new())
            .register_event::<ExtractError>()
            .observe::<AssetEvent<ShaderSource>, _>(Self::on_shader_loaded)
            .sub_app_mut::<RenderApp>()
            .add_sub_phase::<Extract, PostExtract>()
            .add_systems(PostExtract, Self::extract_pipeline_actions)
            .register_event::<ExtractError>();
    }
}

impl ExtractFramework {
    fn extract_pipeline_actions(
        mut extractors: Main<ResMut<PipelineExtractors>>,
        actions: &WorldActions,
    ) {
        actions.extend(extractors.actions.drain(..).map(|(_, action)| action));
    }

    fn on_shader_loaded(
        events: Res<Events<AssetEvent<ShaderSource>>>,
        database: Res<AssetDatabase>,
        mut extractors: ResMut<PipelineExtractors>,
    ) {
        for event in events.iter() {
            let (id, loaded) = match event {
                AssetEvent::Added { id } => (id, true),
                AssetEvent::Modified { id } => (id, true),
                AssetEvent::Unloaded { id, .. } => (id, false),
                AssetEvent::Failed { id, .. } => (id, false),
                _ => continue,
            };

            extractors.shader_updated(LoadPath::Id(*id), loaded);
            let library = database.library().read_blocking();
            if let Some(path) = library.get_path(id) {
                extractors.shader_updated(LoadPath::Path(path.clone()), loaded);
            }
        }
    }
}
