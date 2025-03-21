use crate::{
    app::{
        Process, ProcessAssets, ProcessPipelines, Queue, QueueDraws, QueueViews, Render, RenderApp,
    },
    renderer::{
        graph::RenderGraph, Draw, DrawCalls, DrawPasses, GraphPass, MaterialGraphPass, MaterialPass, MeshDataBuffer, View, ViewBuffer, ViewDrawCalls, ViewEntities, ViewExtractor
    },
    resources::{
        AssetExtractors, DefaultSampler, ExtractError, ExtractInfo, Fallbacks, Mesh, PipelineCache,
        RenderAssetEvent, RenderAssetEvents, RenderAssetExtractor, RenderResource,
        ResourceExtractors, ShaderSource, Texture1d, Texture2d, Texture2dArray, Texture3d,
        TextureCube,
    },
    surface::{RenderSurface, RenderSurfaceTexture},
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
            .add_sub_phase::<Update, Render>()
            .add_resource(RenderSurfaceTexture::default())
            .add_resource(AssetExtractors::default())
            .add_resource(ResourceExtractors::default())
            .add_resource(PipelineCache::default())
            .add_resource(ViewEntities::default())
            .add_non_send_resource(RenderGraph::new())
            .extract_render_asset::<Mesh>()
            .extract_render_asset::<Texture1d>()
            .extract_render_asset::<Texture2d>()
            .extract_render_asset::<Texture2dArray>()
            .extract_render_asset::<Texture3d>()
            .extract_render_asset::<TextureCube>()
            .extract_render_asset::<ShaderSource>()
            .extract_render_resource::<Fallbacks>()
            .extract_render_resource::<DefaultSampler>()
            .register_event::<WindowResized>()
            .observe::<WindowResized, _>(RenderSurface::resize_surface);

        game.observe::<WindowCreated, _>(RenderSurface::extract_surface)
            .observe::<WindowResized, _>(RenderSurface::extract_resize_events)
            .register_asset::<Mesh>()
            .register_asset::<Texture1d>()
            .register_asset::<Texture2d>()
            .register_asset::<Texture2dArray>()
            .register_asset::<Texture3d>()
            .register_asset::<TextureCube>()
            .add_importer::<ShaderSource>();
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        game.scoped_sub_app::<RenderApp>(|game, app| {
            if let Some(extractors) = app.remove_resource::<AssetExtractors>() {
                app.add_systems(Extract, extractors.extract)
                    .add_systems(ProcessAssets, extractors.process);
            }

            app.add_systems(ProcessAssets, ResourceExtractors::process)
                .add_systems(ProcessPipelines, PipelineCache::process)
                .add_systems(Render, RenderGraph::run_graph)
                .add_resource(game.resource::<AssetDatabase>().clone());
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
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self;
    fn extract_view<E: ViewExtractor<V>, V: View>(&mut self) -> &mut Self;
    fn add_draw_calls<D: Draw>(&mut self) -> &mut Self;
    fn add_pass<P: GraphPass>(&mut self) -> &mut Self;
    fn add_material_pass<M: MaterialPass>(&mut self) -> &mut Self;
}

impl RenderAppExt for GameBuilder {
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        game::App::extract_render_asset::<R>(self.sub_app_mut::<RenderApp>());
        self
    }

    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self {
        game::App::extract_render_resource::<R>(self.sub_app_mut::<RenderApp>());
        self
    }

    fn extract_view<E: ViewExtractor<V>, V: View>(&mut self) -> &mut Self {
        game::App::extract_view::<E, V>(self.sub_app_mut::<RenderApp>());
        self
    }

    fn add_draw_calls<D: Draw>(&mut self) -> &mut Self {
        game::App::add_draw_calls::<D>(self.sub_app_mut::<RenderApp>());
        self
    }

    fn add_pass<P: GraphPass>(&mut self) -> &mut Self {
        game::App::add_pass::<P>(self.sub_app_mut::<RenderApp>());
        self
    }

    fn add_material_pass<M: MaterialPass>(&mut self) -> &mut Self {
        game::App::add_material_pass::<M>(self.sub_app_mut::<RenderApp>());
        self
    }
}

impl RenderAppExt for game::App {
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.resource_mut::<AssetExtractors>().add::<R>();
        self.add_resource(RenderAssetEvents::<R>::new());
        self.add_resource(ExtractInfo::<R>::new());
        self.register_event::<ExtractError<R>>();

        self.observe::<AssetEvent<R>, _>(
            |events: Res<Events<AssetEvent<R>>>, mut other: ResMut<RenderAssetEvents<R>>| {
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

        self
    }

    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self {
        self.resource_mut::<ResourceExtractors>().add::<R>();
        self
    }

    fn extract_view<E: ViewExtractor<V>, V: View>(&mut self) -> &mut Self {
        self.add_systems(Extract, ViewBuffer::<V>::extract_views::<E>);
        self.add_systems(QueueViews, ViewBuffer::<V>::queue_views::<E>);
        self.extract_render_resource::<ViewBuffer<V>>();

        self
    }

    fn add_draw_calls<D: Draw>(&mut self) -> &mut Self {
        self.add_resource(DrawCalls::<D>::default());
        self.add_resource(ViewDrawCalls::<D>::default());
        self.add_systems(Extract, DrawCalls::<D>::extract_draws);
        self.add_systems(QueueDraws, DrawCalls::<D>::queue_view_draws);
        self.extract_render_resource::<MeshDataBuffer<D::Mesh>>();
        match self.try_resource_mut::<DrawPasses<D::Pass>>() {
            Some(passes) => passes.add::<D>(),
            None => {
                let mut passes = DrawPasses::<D::Pass>::new();
                passes.add::<D>();
                self.add_resource(passes);
            }
        }

        self.add_material_pass::<D::Pass>();

        // TODO: Add draw call systems and register materials

        self
    }

    fn add_pass<P: GraphPass>(&mut self) -> &mut Self {
        self.non_send_resource_mut::<RenderGraph>().add_pass::<P>();
        self
    }

    fn add_material_pass<M: MaterialPass>(&mut self) -> &mut Self {
        if self.try_resource::<DrawPasses<M>>().is_none() {
            self.add_resource(DrawPasses::<M>::new());
        };

        self.add_pass::<MaterialGraphPass<M>>()
    }
}
