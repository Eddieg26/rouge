use crate::{
    core::{
        ExtractError, RenderAsset, RenderAssetAction, RenderAssetActions, RenderAssetExtractor,
        RenderAssetExtractors, RenderAssetWorld, RenderAssets, RenderDevice, RenderInstance,
        RenderResourceExtractor,
    },
    renderer::graph::{RenderGraph, RenderGraphBuilder},
    resource::{
        mesh::Mesh,
        shader::Shader,
        texture::{
            render::RenderTargetTexture,
            sampler::{Sampler, SamplerDesc},
            target::{RenderTarget, ResizeRenderGraph},
            texture2d::Texture2d,
            RenderTexture,
        },
        Fallbacks, RenderPipelineExtractor, RenderPipelineExtractors, ShaderSource,
    },
    surface::{RenderSurface, RenderSurfaceError, RenderSurfaceTexture},
    RenderResourceExtractors,
};
use asset::{
    database::{events::AssetEvent, AssetDatabase},
    future::block_on,
    io::cache::LoadPath,
    plugin::{AssetExt, AssetPlugin},
    Assets,
};
use ecs::{
    core::{
        resource::{NonSend, Res, ResMut},
        Type,
    },
    event::{Event, Events},
    world::{
        action::{BatchEvents, WorldAction, WorldActions},
        World,
    },
};
use game::{AppTag, ExitGame, Extract, GameBuilder, Main, Plugin, SubActions};
use render_phases::PostExtract;
use window::{
    events::{WindowCreated, WindowResized},
    plugin::WindowPlugin,
    Window,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "Render"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        use render_phases::*;
        game.register_event::<ExtractError>()
            .add_resource(RenderAssetExtractors::new())
            .add_resource(RenderResourceExtractors::new())
            .add_resource(RenderPipelineExtractors::new())
            .add_resource(RenderGraphBuilder::new())
            .observe::<WindowCreated, _>(create_render_surface)
            .observe::<WindowResized, _>(extract_resize_events)
            .observe::<AssetEvent<RenderTargetTexture>, _>(
                |events: Res<Events<AssetEvent<RenderTargetTexture>>>,
                 actions: SubActions<RenderApp>| {
                    if events
                        .iter()
                        .any(|event| !matches!(event, AssetEvent::Imported { .. }))
                    {
                        actions.add(ResizeRenderGraph);
                    }
                },
            )
            .add_sub_app::<RenderApp>()
            .add_resource(RenderSurfaceTexture::default())
            .add_phase::<PreRender>()
            .add_phase::<Render>()
            .add_phase::<PostRender>()
            .add_phase::<Present>()
            .add_extract_phase::<PostExtract>()
            .add_systems(PreRender, set_surface_texture)
            .add_systems(Render, run_render_graph)
            .add_systems(Present, present_surface_texture)
            .add_systems(PostExtract, extract_render_pipelines)
            .register_event::<WindowResized>()
            .register_event::<SurfaceCreated>()
            .register_event::<ResizeRenderGraph>()
            .register_event::<ExtractError>()
            .observe::<WindowResized, _>(on_window_resized)
            .observe::<ResizeRenderGraph, _>(on_resize_render_graph);

        game.add_importer::<ShaderSource>();
        game.register_render_asset::<Sampler>();
        game.add_render_asset_extractor::<Mesh>();
        game.add_render_asset_extractor::<Texture2d>();
        game.add_render_asset_extractor::<RenderTarget>();
        game.add_render_asset_extractor::<Shader>();
        game.add_render_resource_extractor::<Fallbacks>();
        game.add_render_resource_extractor::<RenderGraph>();
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        let extractors = game
            .remove_resource::<RenderResourceExtractors>()
            .unwrap_or(RenderResourceExtractors::new());
        game.sub_app_mut::<RenderApp>()
            .add_systems(Extract, extract_resources)
            .add_resource(extractors);

        if let Some(extractors) = game.remove_resource::<RenderAssetExtractors>() {
            let systems = extractors.build();
            game.sub_app_mut::<RenderApp>()
                .add_systems(Extract, systems);
        }

        if let Some(extractors) = game.remove_resource::<RenderPipelineExtractors>() {
            let app = game.sub_app_mut::<RenderApp>();
            app.add_resource(extractors);
        }

        match game.remove_resource::<RenderGraphBuilder>() {
            Some(builder) => game.sub_app_mut::<RenderApp>().add_resource(builder),
            None => game
                .sub_app_mut::<RenderApp>()
                .add_resource(RenderGraphBuilder::new()),
        };
    }

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::default();
        plugins.add(AssetPlugin);
        plugins.add(WindowPlugin);

        plugins
    }
}

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}

pub mod render_phases {
    use ecs::system::schedule::Phase;

    pub struct PreRender;
    impl Phase for PreRender {}
    pub struct Render;
    impl Phase for Render {}
    pub struct PostRender;
    impl Phase for PostRender {}
    pub struct Present;
    impl Phase for Present {}
    pub struct PostExtract;
    impl Phase for PostExtract {}
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

fn extract_resources(world: &World, mut extractors: ResMut<RenderResourceExtractors>) {
    extractors.extract(world);
}

fn extract_resize_events(events: Res<Events<WindowResized>>, actions: SubActions<RenderApp>) {
    actions.add(BatchEvents::new(events.iter().copied()));
}

fn extract_render_pipelines(
    world: &World,
    mut extractors: ResMut<RenderPipelineExtractors>,
    actions: Main<Res<RenderAssetActions<ShaderSource>>>,
    database: Main<Res<AssetDatabase>>,
) {
    for action in actions.iter() {
        let (id, loaded) = match action {
            RenderAssetAction::Added { id } => (id, true),
            RenderAssetAction::Modified { id } => (id, true),
            RenderAssetAction::Removed { id } => (id, false),
            _ => continue,
        };

        let library = database.library().read_arc_blocking();
        if let Some(path) = library.get_path(&id).cloned() {
            extractors.shader_updated(world, LoadPath::Path(path), loaded);
        }

        extractors.shader_updated(world, LoadPath::Id(*id), loaded);
    }
}

fn on_window_resized(
    events: Res<Events<WindowResized>>,
    device: Res<RenderDevice>,
    mut surface: ResMut<RenderSurface>,
    mut targets: ResMut<RenderAssets<RenderTarget>>,
    mut updates: ResMut<Events<ResizeRenderGraph>>,
) {
    if let Some(event) = events.last() {
        surface.resize(&device, event.size.width, event.size.height);

        if let Some(target) = targets.get_mut(&RenderSurface::ID) {
            target.width = event.size.width;
            target.height = event.size.height;
            updates.add(ResizeRenderGraph);
        }
    }
}

pub struct SurfaceCreated;
impl Event for SurfaceCreated {}

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

pub trait RenderAppExt {
    fn register_render_asset<R: RenderAsset>(&mut self) -> &mut Self;
    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn add_render_asset_dependency<R: RenderAssetExtractor, D: RenderAssetExtractor>(
        &mut self,
    ) -> &mut Self;
    fn add_render_resource_extractor<R: RenderResourceExtractor>(&mut self) -> &mut Self;
    fn add_render_pipeline_extractor<R: RenderPipelineExtractor>(&mut self) -> &mut Self;
}

impl RenderAppExt for GameBuilder {
    fn register_render_asset<R: RenderAsset>(&mut self) -> &mut Self {
        match R::world() {
            RenderAssetWorld::Main => {
                if !self.has_resource::<RenderAssets<R>>() {
                    self.add_resource(RenderAssets::<R>::new());
                }
            }
            RenderAssetWorld::Render => {
                let app = self.sub_app_mut::<RenderApp>();

                if !app.has_resource::<RenderAssets<R>>() {
                    app.add_resource(RenderAssets::<R>::new());
                }
            }
        }

        self
    }

    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.register_asset::<R::Source>();
        self.register_render_asset::<R::Asset>();
        self.resource_mut::<RenderAssetExtractors>().add::<R>();
        if !self.has_resource::<RenderAssetActions<R::Source>>() {
            self.add_resource(RenderAssetActions::<R::Source>::new());

            self.observe::<AssetEvent<R::Source>, _>(
                |events: Res<Events<AssetEvent<R::Source>>>,
                 mut actions: ResMut<RenderAssetActions<R::Source>>| {
                    for event in events.iter() {
                        match event {
                            AssetEvent::Added { id } | AssetEvent::Loaded { id } => {
                                actions.add(RenderAssetAction::Added { id: *id })
                            }
                            AssetEvent::Unloaded { id, .. } => {
                                actions.add(RenderAssetAction::Removed { id: *id })
                            }
                            AssetEvent::Modified { id } => {
                                actions.add(RenderAssetAction::Modified { id: *id })
                            }
                            AssetEvent::Failed { id, .. } => {
                                actions.add(RenderAssetAction::Removed { id: *id })
                            }
                            AssetEvent::Imported { .. } => continue,
                        }
                    }
                },
            );
        }

        self.sub_app_mut::<RenderApp>().add_systems(
            PostExtract,
            |mut assets: Main<ResMut<Assets<R::Source>>>,
             mut actions: Main<ResMut<RenderAssetActions<R::Source>>>| {
                for action in actions.iter() {
                    if let RenderAssetAction::Removed { id } = action {
                        assets.remove(id);
                    }
                }

                actions.clear();
            },
        );

        self
    }

    fn add_render_asset_dependency<R: RenderAssetExtractor, D: RenderAssetExtractor>(
        &mut self,
    ) -> &mut Self {
        let extractors = self.resource_mut::<RenderAssetExtractors>();
        extractors.add_dependency::<R, D>();

        self
    }

    fn add_render_resource_extractor<R: RenderResourceExtractor>(&mut self) -> &mut Self {
        let extractors = self.resource_mut::<RenderResourceExtractors>();
        extractors.add::<R>();
        self
    }

    fn add_render_pipeline_extractor<R: RenderPipelineExtractor>(&mut self) -> &mut Self {
        self.resource_mut::<RenderPipelineExtractors>().add::<R>();
        self.observe::<R::Trigger, _>(
            |world: &World, mut extractors: ResMut<RenderPipelineExtractors>| {
                let world = unsafe { world.cell() };
                if let Some(state) = extractors.get_mut(Type::of::<R>()) {
                    let prev = state.event_triggered;
                    state.event_triggered = true;
                    if !prev && state.vertex_shader_loaded && state.fragment_shader_loaded {
                        (state.create_pipeline)(world);
                    }
                }
            },
        );
        self
    }
}
