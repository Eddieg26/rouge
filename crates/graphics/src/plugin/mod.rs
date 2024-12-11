use crate::resource::extract::{PipelineExtractor, PipelineExtractors};
use crate::{
    core::{
        RenderAsset, RenderAssetAction, RenderAssetActions, RenderAssetExtractor,
        RenderAssetExtractors, RenderAssetWorld, RenderAssets, RenderResourceExtractor,
    },
    renderer::graph::{RenderGraph, RenderGraphBuilder},
    resource::{
        mesh::Mesh,
        shader::Shader,
        texture::{sampler::Sampler, target::RenderTarget, texture2d::Texture2d},
        Fallbacks, ShaderSource,
    },
    RenderResourceExtractors,
};
use crate::{
    BatchDrawCalls, BatchDrawExtractor, DrawCalls, DrawExtractor, RenderView, ViewExtractor,
};
use asset::{
    database::events::AssetEvent,
    plugin::{AssetExt, AssetPlugin},
    Assets,
};
use ecs::system::StaticArg;
use ecs::{
    core::resource::{Res, ResMut},
    event::Events,
    world::World,
};
use frameworks::{CreateSurfaceFramework, ExtractFramework, RenderFramework, ResizeFramework};
use game::{Extract, GameBuilder, Main, Plugin};
use phases::PostExtract;
use window::plugin::WindowPlugin;

pub mod frameworks;
pub mod phases;

pub use frameworks::RenderApp;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "Render"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        game.add_framework(RenderFramework)
            .add_framework(ResizeFramework)
            .add_framework(CreateSurfaceFramework)
            .add_framework(ExtractFramework)
            .add_importer::<ShaderSource>()
            .register_render_asset::<Sampler>()
            .add_render_asset_extractor::<Mesh>()
            .add_render_asset_extractor::<Texture2d>()
            .add_render_asset_extractor::<RenderTarget>()
            .add_render_asset_extractor::<Shader>()
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
    fn add_pipeline_extractor<P: PipelineExtractor>(&mut self) -> &mut Self;
    fn add_view_extractor<V: ViewExtractor>(&mut self) -> &mut Self;
    fn add_draw_call_extractor<D: DrawExtractor>(&mut self) -> &mut Self;
    fn add_batch_draw_call_extractor<D: BatchDrawExtractor>(&mut self) -> &mut Self;
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

    fn add_pipeline_extractor<P: PipelineExtractor>(&mut self) -> &mut Self {
        self.resource_mut::<PipelineExtractors>()
            .add_extractor::<P>();
        self
    }

    fn add_view_extractor<V: ViewExtractor>(&mut self) -> &mut Self {
        self.register_render_asset::<RenderView<V::View>>();
        self.add_systems(
            Extract,
            |mut views: ResMut<RenderAssets<RenderView<V::View>>>, arg: StaticArg<V::Arg>| {
                V::extract(&mut views, arg.into_inner());
            },
        )
    }

    fn add_draw_call_extractor<D: DrawExtractor>(&mut self) -> &mut Self {
        self.sub_app_mut::<RenderApp>()
            .add_resource(DrawCalls::<D::Draw>::new())
            .add_systems(
                Extract,
                |mut calls: ResMut<DrawCalls<D::Draw>>, arg: StaticArg<D::Arg>| {
                    D::extract(&mut calls, arg.into_inner());
                },
            );

        self
    }

    fn add_batch_draw_call_extractor<D: BatchDrawExtractor>(&mut self) -> &mut Self {
        self.sub_app_mut::<RenderApp>()
            .add_resource(BatchDrawCalls::<D::Draw>::new())
            .add_systems(
                Extract,
                |mut calls: ResMut<BatchDrawCalls<D::Draw>>, arg: StaticArg<D::Arg>| {
                    D::extract(&mut calls, arg.into_inner());
                },
            );

        self
    }
}
