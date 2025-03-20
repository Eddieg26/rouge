use crate::{
    app::{Process, Queue, Render, RenderApp},
    renderer::graph::RenderGraph,
    resources::{
        AssetExtractors, PipelineCache, RenderAssetEvent, RenderAssetEvents, RenderAssetExtractor,
        RenderResource, ResourceExtractors,
    },
    surface::RenderSurface,
};
use asset::database::events::AssetEvent;
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
            .add_resource(AssetExtractors::new())
            .add_resource(ResourceExtractors::new())
            .add_resource(PipelineCache::new())
            .add_non_send_resource(RenderGraph::new())
            .add_sub_phase::<Update, Process>()
            .add_sub_phase::<Update, Queue>()
            .add_sub_phase::<Update, Render>()
            .register_event::<WindowResized>()
            .observe::<WindowResized, _>(RenderSurface::resize_surface);

        game.observe::<WindowCreated, _>(RenderSurface::extract_surface);
        game.observe::<WindowResized, _>(RenderSurface::extract_resize_events);
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        game.scoped_sub_app::<RenderApp>(|_, app| {
            if let Some(extractors) = app.remove_resource::<AssetExtractors>() {
                app.add_systems(Extract, extractors.extract);
                app.add_systems(Process, extractors.process);
            }

            app.add_systems(Process, ResourceExtractors::process);
            app.add_systems(Process, PipelineCache::process);

            app.add_systems(Render, RenderGraph::run_graph);
        });
    }

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::default();
        plugins.add(WindowPlugin);

        plugins
    }
}

pub trait RenderAppExt {
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self;
    fn extract_render_resource<R: RenderResource>(&mut self) -> &mut Self;
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
}

impl RenderAppExt for game::App {
    fn extract_render_asset<R: RenderAssetExtractor>(&mut self) -> &mut Self {
        self.add_resource(RenderAssetEvents::<R>::new());
        self.resource_mut::<AssetExtractors>().add::<R>();

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
}
