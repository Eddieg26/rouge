use crate::{
    extract::{
        RenderAsset, RenderAssetAction, RenderAssetActions, RenderAssetExtractor,
        RenderAssetExtractors, RenderAssetWorld, RenderAssets, RenderResourceExtractor,
        RenderResourceExtractors,
    },
    renderer::graph::{RenderGraph, RenderGraphBuilder},
    resource::{
        mesh::Mesh, texture::Texture2d, Fallbacks, RenderTexture, ShaderSource, Texture2dArray,
    },
    RenderApp,
};
use asset::{
    database::events::AssetEvent,
    plugin::{AssetExt, AssetPlugin},
};
use ecs::{
    core::resource::{Res, ResMut},
    event::Events,
    world::World,
};
use frameworks::{ExtractFramework, RenderFramework, ResizeFramework};
use game::{Extract, GameBuilder, Plugin};
use window::plugin::WindowPlugin;

pub mod frameworks;
pub mod phases;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "Render"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        game.add_framework(RenderFramework)
            .add_framework(ResizeFramework)
            .add_framework(ExtractFramework)
            .add_importer::<ShaderSource>()
            .add_render_asset_extractor::<Mesh>()
            .add_render_asset_extractor::<Texture2d>()
            .add_render_asset_extractor::<Texture2dArray>()
            .add_render_asset_extractor::<RenderTexture>()
            .add_render_asset_extractor::<ShaderSource>()
            .add_render_resource_extractor::<Fallbacks>()
            .add_render_resource_extractor::<RenderGraph>();
    }

    fn finish(&mut self, game: &mut game::GameBuilder) {
        let extractors = game
            .remove_resource::<RenderResourceExtractors>()
            .unwrap_or_default();

        game.sub_app_mut::<RenderApp>()
            .add_systems(Extract, extract_resources)
            .add_resource(extractors);

        if let Some(extractors) = game.remove_resource::<RenderAssetExtractors>() {
            game.sub_app_mut::<RenderApp>()
                .add_systems(Extract, extractors.build());
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

fn extract_resources(world: &World, mut extractors: ResMut<RenderResourceExtractors>) {
    extractors.extract(world);
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
        self.register_asset::<R>();
        self.register_render_asset::<R::Target>();
        self.resource_mut::<RenderAssetExtractors>().add::<R>();
        if !self.has_resource::<RenderAssetActions<R>>() {
            self.add_resource(RenderAssetActions::<R>::new());

            self.observe::<AssetEvent<R>, _>(
                |events: Res<Events<AssetEvent<R>>>, mut actions: ResMut<RenderAssetActions<R>>| {
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
}
