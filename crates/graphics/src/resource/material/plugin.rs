use super::{
    globals::GlobalLayout, Material, MaterialInstance, MaterialMetadata, MaterialPipelines,
    MeshPipelineData,
};
use crate::{
    plugin::{RenderApp, RenderAppExt},
    resource::{
        material::{MeshPipeline, Metadata},
        MaterialPipelineDesc, MaterialType, Shader,
    },
    surface::RenderSurface,
    RenderAssetExtractor, RenderAssets, RenderDevice,
};
use asset::{database::AssetDatabase, io::cache::LoadPath};
use ecs::system::{
    unlifetime::{ReadRes, WriteRes},
    StaticArg,
};
use game::{GameBuilder, Main, Plugin};

pub struct BaseMaterialPlugin;

impl Plugin for BaseMaterialPlugin {
    fn name(&self) -> &'static str {
        "BaseMaterialPlugin"
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_render_asset_extractor::<GlobalLayout>();
    }

    fn run(&mut self, _game: &mut GameBuilder) {}

    fn finish(&mut self, _game: &mut GameBuilder) {}
}

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

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_render_asset_extractor::<M>();
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
    type Arg = StaticArg<
        'static,
        (
            ReadRes<RenderDevice>,
            ReadRes<RenderSurface>,
            ReadRes<RenderAssets<Shader>>,
            ReadRes<GlobalLayout>,
            ReadRes<MeshPipelineData<M::Pipeline>>,
            ReadRes<MaterialMetadata<M::Meta>>,
            WriteRes<MaterialPipelines>,
            Main<'static, ReadRes<AssetDatabase>>,
            M::Arg,
        ),
    >;

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
        ) = arg.inner_mut();

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
        let (_, _, _, _, _, _, pipelines, _, _) = arg.inner_mut();

        if let Some(instance) = assets.remove(id) {
            pipelines.remove_dependency(&instance.ty, id);
        }
    }
}
