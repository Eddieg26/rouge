use crate::{
    core::{
        ExtractError, RenderAsset, RenderAssetAction, RenderAssetActions, RenderAssetExtractor,
        RenderAssetExtractors, RenderAssetWorld, RenderAssets, RenderDevice, RenderInstance,
        RenderResourceExtractor,
    },
    renderer::graph::{RenderGraph, RenderGraphBuilder},
    resources::{
        mesh::Mesh,
        shader::Shader,
        texture::{sampler::Sampler, texture2d::Texture2d, RenderTexture},
    },
    surface::{
        target::{RenderTarget, RenderTargetsUpdated},
        RenderSurface, RenderSurfaceError, RenderSurfaceTexture,
    },
};
use asset::{
    future::block_on,
    plugin::{AssetExt, AssetPlugin},
    Assets,
};
use ecs::{
    core::resource::{NonSend, Res, ResMut},
    event::{Event, Events},
    system::{unlifetime::ReadRes, StaticArg},
    world::{
        access::Removed,
        action::{WorldAction, WorldActions},
        builtin::actions::AddResource,
        World,
    },
};
use game::{AppTag, ExitGame, Extract, GameBuilder, Main, Plugin, SubActions};
use render_phases::PostExtract;
use spatial::size::Size;
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
            .add_resource(RenderGraphBuilder::new())
            .observe::<WindowCreated, _>(create_render_surface)
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
            .register_event::<WindowResized>()
            .register_event::<SurfaceCreated>()
            .register_event::<RenderTargetsUpdated>()
            .register_event::<ExtractError>()
            .observe::<WindowResized, _>(on_window_resized)
            .observe::<RenderTargetsUpdated, _>(on_render_targets_updated);

        game.register_render_asset::<Sampler>();
        game.add_render_asset_extractor::<Mesh>();
        game.add_render_asset_extractor::<Texture2d>();
        game.add_render_asset_extractor::<RenderTarget>();
        game.add_render_asset_extractor::<Shader>();
        game.add_render_resource_extractor::<RenderGraph>();
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        if let Some(extractors) = game.remove_resource::<RenderAssetExtractors>() {
            let configs = extractors.build();
            game.sub_app_mut::<RenderApp>()
                .add_systems(Extract, configs);
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
            size: Size::new(surface.width(), surface.height()),
            format: surface.format(),
            color: RenderSurface::ID.to(),
            sampler: RenderSurface::ID.to(),
        };

        world.get_mut().add_resource(surface);
        world.get_mut().add_resource(device);

        world
            .resource_mut::<RenderAssets<RenderTarget>>()
            .add(RenderSurface::ID.to(), target);

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

fn run_render_graph(mut graph: ResMut<RenderGraph>, world: &World) {
    graph.run(world);
}

fn present_surface_texture(mut surface_texture: ResMut<RenderSurfaceTexture>) {
    surface_texture.present();
}

fn on_window_resized(
    events: Res<Events<WindowResized>>,
    device: Res<RenderDevice>,
    mut surface: ResMut<RenderSurface>,
    mut targets: ResMut<RenderAssets<RenderTarget>>,
    mut updates: ResMut<Events<RenderTargetsUpdated>>,
) {
    if let Some(event) = events.last() {
        let size = Size::new(event.size.width, event.size.height);
        surface.resize(&device, size);

        if let Some(target) = targets.get_mut(&RenderSurface::ID) {
            target.size = size;
            updates.add(RenderTargetsUpdated);
        }
    }
}

pub struct SurfaceCreated;
impl Event for SurfaceCreated {}

impl RenderResourceExtractor for RenderGraph {
    type Resource = RenderGraph;
    type Arg = StaticArg<
        'static,
        (
            ReadRes<RenderDevice>,
            ReadRes<RenderAssets<RenderTarget>>,
            Removed<RenderGraphBuilder>,
        ),
    >;

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self::Resource, ExtractError> {
        let (device, targets, builder) = arg.into_inner();
        if let Some(builder) = builder.into_inner() {
            Ok(builder.build(&device, targets.max_size()))
        } else {
            Ok(RenderGraph::default())
        }
    }
}

fn on_render_targets_updated(
    targets: Res<RenderAssets<RenderTarget>>,
    device: Res<RenderDevice>,
    mut graph: ResMut<RenderGraph>,
) {
    graph.resize(&device, targets.max_size());
}

pub trait RenderAppExt {
    fn register_render_asset<R: RenderAsset>(&mut self) -> &mut Self;
    fn add_render_asset_extractor<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn add_render_asset_dependency<R: RenderAssetExtractor, D: RenderAssetExtractor>(
        &mut self,
    ) -> &mut Self;
    fn add_render_resource_extractor<R: RenderResourceExtractor>(&mut self) -> &mut Self;
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
        }

        self.add_systems(
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
        let app = self.sub_app_mut::<RenderApp>();
        app.observe::<SurfaceCreated, _>(
            |arg: StaticArg<R::Arg>,
             actions: &WorldActions,
             mut errors: ResMut<Events<ExtractError>>| {
                let arg = arg.into_inner();
                match R::extract(arg) {
                    Ok(resource) => actions.add(AddResource::new(resource)),
                    Err(error) => errors.add(error),
                }
            },
        );
        self
    }
}
