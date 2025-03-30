use crate::{
    DrawFunctions, DrawMesh, DrawPipline, ExtractedViews, MaterialRef, MeshData, PostRender,
    PreRender, ProcessResources, RenderAssets, ViewDrawCalls,
    app::{
        Process, ProcessAssets, ProcessPipelines, Queue, QueueDraws, QueueViews, Render, RenderApp,
    },
    renderer::{
        Draw, Draws, MeshDataBuffer, RenderGraphPass, Renderer, ViewBuffer, ViewData,
        graph::RenderGraph,
    },
    resources::{
        AssetExtractors, ExtractError, ExtractInfo, Fallbacks, Material, Mesh, PipelineCache,
        RenderAssetEvent, RenderAssetEvents, RenderAssetExtractor, RenderResource,
        ResourceExtractors, ShaderSource, Texture,
    },
    surface::RenderSurface,
};
use asset::{
    database::{AssetDatabase, events::AssetEvent},
    plugin::{AssetExt, AssetPlugin},
};
use ecs::{Res, ResMut, event::Events};
use game::{Extract, GameBuilder, Plugin, Update};
use window::{
    events::{WindowCreated, WindowResized},
    plugin::WindowPlugin,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "Render"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        game.add_sub_app::<RenderApp>()
            .add_sub_phase::<Update, Process>()
            .add_sub_phase::<Update, Queue>()
            .add_sub_phase::<Update, PreRender>()
            .add_sub_phase::<Update, Render>()
            .add_sub_phase::<Update, PostRender>()
            .add_resource(AssetExtractors::default())
            .add_resource(ResourceExtractors::default())
            .add_resource(PipelineCache::default())
            .add_non_send_resource(RenderGraph::new())
            .register_event::<WindowResized>()
            .observe::<WindowResized, _>(RenderSurface::resize_surface);

        game.extract_render_resource::<Fallbacks>()
            .extract_render_asset::<Texture>()
            .extract_render_asset::<Mesh>()
            .extract_render_asset::<ShaderSource>()
            .add_importer::<ShaderSource>()
            .observe::<WindowCreated, _>(RenderSurface::extract_surface)
            .observe::<WindowResized, _>(RenderSurface::extract_resize_events);
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        game.scoped_sub_app::<RenderApp>(|game, app| {
            app.add_systems(ProcessResources, ResourceExtractors::process)
                .add_systems(ProcessPipelines, PipelineCache::process)
                .add_systems(Render, RenderGraph::run_graph)
                .add_resource(game.resource::<AssetDatabase>().clone());

            if let Some(extractors) = app.remove_resource::<AssetExtractors>() {
                app.add_systems(Extract, extractors.extract)
                    .add_systems(ProcessAssets, extractors.process);
            }
        });
    }

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::default();
        plugins.add(WindowPlugin);
        plugins.add(AssetPlugin);

        plugins
    }
}

pub trait RenderAppExt {
    fn add_pass<P: RenderGraphPass>(&mut self, pass: P) -> &mut Self;
    fn add_view<V: ViewData>(&mut self) -> &mut Self;
    fn add_draw<M: Material, D: Draw<Material = M>>(&mut self) -> &mut Self;
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self;
}

impl RenderAppExt for GameBuilder {
    fn add_pass<P: RenderGraphPass>(&mut self, pass: P) -> &mut Self {
        self.sub_app_mut::<RenderApp>()
            .non_send_resource_mut::<RenderGraph>()
            .add_pass::<P>(pass);
        self
    }

    fn add_view<V: ViewData>(&mut self) -> &mut Self {
        self.add_plugin(ViewPlugin::<V>::new())
    }

    fn add_draw<M: Material, D: Draw<Material = M>>(&mut self) -> &mut Self {
        self.add_plugin(DrawPlugin::<M, D>::new())
    }

    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.scoped_sub_app::<RenderApp>(|game, app| {
            if app.resource_mut::<AssetExtractors>().add::<R>() {
                app.register_event::<ExtractError<R>>();
                app.add_resource(ExtractInfo::<R>::new());
                app.add_resource(RenderAssets::<R::RenderAsset>::new());

                game.add_resource(RenderAssetEvents::<R>::new());
                game.observe::<AssetEvent<R>, _>(
                    |events: Res<Events<AssetEvent<R>>>,
                     mut other: ResMut<RenderAssetEvents<R>>| {
                        for event in events.iter() {
                            let event = match event {
                                AssetEvent::Added { id } => RenderAssetEvent::Added(*id),
                                AssetEvent::Unloaded { id, .. } => RenderAssetEvent::Removed(*id),
                                AssetEvent::Loaded { id } => RenderAssetEvent::Modified(*id),
                                AssetEvent::Modified { id } => RenderAssetEvent::Modified(*id),
                                AssetEvent::Failed { id, .. } => RenderAssetEvent::Removed(*id),
                                _ => continue,
                            };

                            other.add(event);
                        }
                    },
                );
            }
        });

        self.register_asset::<R>()
    }

    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self {
        self.sub_app_mut::<RenderApp>()
            .register_event::<ExtractError<()>>()
            .resource_mut::<ResourceExtractors>()
            .add::<R>();
        self
    }
}

pub struct ViewPlugin<V: ViewData>(std::marker::PhantomData<V>);
impl<V: ViewData> ViewPlugin<V> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<V: ViewData> Plugin for ViewPlugin<V> {
    fn name(&self) -> &'static str {
        std::any::type_name::<V>()
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.sub_app_mut::<RenderApp>()
            .add_resource(ExtractedViews::<V>::default())
            .add_systems(Extract, ViewBuffer::<V>::extract)
            .add_systems(QueueViews, ViewBuffer::<V>::queue)
            .add_systems(PreRender, ViewBuffer::<V>::update_views)
            .add_systems(PostRender, ViewBuffer::<V>::clear_views);

        game.extract_render_resource::<ViewBuffer<V>>();
    }
}

pub struct DrawPlugin<M: Material, D: Draw<Material = M>>(std::marker::PhantomData<(D, M)>);
impl<M: Material, D: Draw<Material = M>> DrawPlugin<M, D> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<M: Material, D: Draw<Material = M>> Plugin for DrawPlugin<M, D> {
    fn name(&self) -> &'static str {
        std::any::type_name::<D>()
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.scoped_sub_app::<RenderApp>(|_, app| {
            app.add_resource(Draws::<D>::default());
            app.add_systems(Extract, Draws::<D>::extract);
            app.add_systems(QueueDraws, ViewDrawCalls::<D::View, M::Phase>::queue::<D>);
            app.add_systems(PostRender, Draws::<D>::clear_draws);
            match app.try_resource_mut::<DrawFunctions<D::View>>() {
                Some(functions) => functions.add::<D>(),
                None => {
                    let mut functions = DrawFunctions::<D::View>::default();
                    functions.add::<D>();
                    app.add_resource(functions);
                }
            }
        });

        game.load_asset::<ShaderSource>(D::Renderer::shader().into());
        game.load_asset::<ShaderSource>(D::Material::shader().into());

        game.register_asset::<D::Material>();
        game.register::<MaterialRef<D::Material>>();
        game.extract_render_asset::<D::Material>();
        game.extract_render_resource::<DrawPipline<D>>();
    }

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::new();
        plugins.add(RenderPlugin);
        plugins.add(ViewPlugin::<D::View>::new());
        plugins.add(MeshDataPlugin::<DrawMesh<D>>::new());

        plugins
    }
}

pub struct MeshDataPlugin<M: MeshData>(std::marker::PhantomData<M>);
impl<M: MeshData> MeshDataPlugin<M> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<M: MeshData> Plugin for MeshDataPlugin<M> {
    fn name(&self) -> &'static str {
        std::any::type_name::<M>()
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.extract_render_resource::<MeshDataBuffer<M>>();
        game.sub_app_mut::<RenderApp>()
            .add_systems(PreRender, MeshDataBuffer::<M>::update_mesh_buffer)
            .add_systems(PostRender, MeshDataBuffer::<M>::clear_mesh_buffer);
    }
}
