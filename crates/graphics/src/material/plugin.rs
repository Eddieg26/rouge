use super::{Material, MaterialLight, MaterialMesh, MaterialPipeline, MaterialView, MeshRenderer};
use crate::{
    plugin::{RenderAppExt, RenderPlugin},
    resource::ShaderSource,
};
use asset::plugin::AssetExt;
use game::{GameBuilder, Plugin};

pub struct MaterialPlugin<M: Material> {
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> MaterialPlugin<M> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: Material> Plugin for MaterialPlugin<M> {
    fn name(&self) -> &'static str {
        std::any::type_name::<M>()
    }

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::default();
        plugins.add(RenderPlugin);
        plugins
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_render_asset_extractor::<M>()
            .add_render_resource_extractor::<MaterialView<M>>()
            .add_render_resource_extractor::<MaterialMesh<M>>()
            .add_render_resource_extractor::<MaterialLight<M>>()
            .add_render_asset_dependency::<M, ShaderSource>()
            .add_render_resource_extractor::<MaterialPipeline<M>>()
            .load_asset::<ShaderSource>(M::shader())
            .load_asset::<ShaderSource>(M::Renderer::shader());
    }
}

pub trait MaterialAppExt: 'static {
    fn add_material<M: Material>(&mut self) -> &mut Self;
}

impl MaterialAppExt for GameBuilder {
    fn add_material<M: Material>(&mut self) -> &mut Self {
        self.add_plugin(MaterialPlugin::<M>::new())
    }
}
