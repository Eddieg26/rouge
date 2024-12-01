use super::{
    globals::{GlobalLayout, Globals},
    Material, MaterialInstance, MaterialMetadata, MaterialPipelines, MeshPipelineData,
};
use crate::{
    plugin::{RenderApp, RenderAppExt, RenderPlugin},
    resource::{
        material::{MeshPipeline, Metadata},
        MaterialPipelineDesc, MaterialType, Shader, ShaderSource,
    },
    surface::RenderSurface,
    RenderAssetExtractor, RenderAssets, RenderDevice, View,
};
use asset::{database::AssetDatabase, io::cache::LoadPath, plugin::AssetExt};
use ecs::system::unlifetime::{ReadRes, WriteRes};
use game::{GameBuilder, Main, Plugin};

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
            .add_render_resource_extractor::<GlobalLayout>()
            .add_render_resource_extractor::<MaterialMetadata<M::Meta>>()
            .add_render_resource_extractor::<MeshPipelineData<M::Pipeline>>()
            .load_asset::<ShaderSource>(M::shader())
            .load_asset::<ShaderSource>(M::Pipeline::shader());
    }

    fn finish(&mut self, game: &mut GameBuilder) {
        let app = game.sub_app_mut::<RenderApp>();
        if !app.has_resource::<MaterialPipelines>() {
            app.add_resource(MaterialPipelines::new());
        }
    }
}

impl<M: Material> RenderAssetExtractor for M {
    type Source = M;
    type Asset = MaterialInstance;
    type Arg = (
        ReadRes<RenderDevice>,
        ReadRes<RenderSurface>,
        ReadRes<RenderAssets<Shader>>,
        ReadRes<GlobalLayout>,
        ReadRes<MeshPipelineData<M::Pipeline>>,
        ReadRes<MaterialMetadata<M::Meta>>,
        WriteRes<MaterialPipelines>,
        Main<'static, ReadRes<AssetDatabase>>,
        M::Arg,
    );

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::ExtractError> {
        let (
            device,
            surface,
            shaders,
            global_layout,
            surface_pipeline,
            metadata,
            pipelines,
            database,
            material_arg,
        ) = arg;

        let ty = MaterialType::of::<M>();
        let layout = match pipelines.get(&ty).map(|p| p.layout()) {
            Some(layout) => layout.clone(),
            None => {
                let vertex_shader = match M::Pipeline::shader().into() {
                    LoadPath::Id(id) => match shaders.get(&id.into()) {
                        Some(shader) => shader,
                        None => return Err(crate::ExtractError::MissingAsset),
                    },
                    LoadPath::Path(path) => {
                        let library = database.library().read_blocking();
                        let id = match library.get_id(&path) {
                            Some(id) => id,
                            None => return Err(crate::ExtractError::MissingAsset),
                        };

                        match shaders.get(&id.into()) {
                            Some(shader) => shader,
                            None => return Err(crate::ExtractError::MissingAsset),
                        }
                    }
                };

                let fragment_shader = match M::Pipeline::shader().into() {
                    LoadPath::Id(id) => match shaders.get(&id.into()) {
                        Some(shader) => shader,
                        None => return Err(crate::ExtractError::MissingAsset),
                    },
                    LoadPath::Path(path) => {
                        let library = database.library().read_blocking();
                        let id = match library.get_id(&path) {
                            Some(id) => id,
                            None => return Err(crate::ExtractError::MissingAsset),
                        };

                        match shaders.get(&id.into()) {
                            Some(shader) => shader,
                            None => return Err(crate::ExtractError::MissingAsset),
                        }
                    }
                };

                let desc = MaterialPipelineDesc {
                    format: surface.format(),
                    depth_format: Some(surface.depth_format()),
                    global_layout: &global_layout,
                    mesh: &surface_pipeline.data,
                    metadata: &metadata.metadata,
                    vertex_shader,
                    fragment_shader,
                };

                pipelines.add::<M>(device, desc, *id)
            }
        };

        let binding = match source.bind_group(device, &layout, material_arg) {
            Ok(binding) => binding,
            Err(error) => return Err(error.into()),
        };

        let instance = MaterialInstance {
            ty,
            binding,
            mode: M::mode(),
            model: M::Meta::model(),
        };

        Ok(instance)
    }

    fn remove(
        id: &asset::AssetId,
        assets: &mut crate::RenderAssets<Self::Asset>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (_, _, _, _, _, _, pipelines, _, _) = arg;

        if let Some(instance) = assets.remove(id) {
            pipelines.remove_dependency(&instance.ty, id);
        }
    }
}

pub trait MaterialAppExt: 'static {
    fn add_material<M: Material>(&mut self) -> &mut Self;
    fn add_view_globals<V: View>(&mut self) -> &mut Self;
}

impl MaterialAppExt for GameBuilder {
    fn add_material<M: Material>(&mut self) -> &mut Self {
        self.add_plugin(MaterialPlugin::<M>::new())
    }

    fn add_view_globals<V: View>(&mut self) -> &mut Self {
        self.add_render_resource_extractor::<Globals<V>>()
    }
}
