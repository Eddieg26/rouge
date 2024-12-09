use super::{Material, MaterialGlobals, MaterialInstance, MaterialMesh, MaterialPipelines};
use crate::{
    plugin::{RenderApp, RenderAppExt, RenderPlugin},
    resource::{
        extract::{ExtractPipeline, PipelineExtractor},
        material::{MaterialModel, MeshPipeline},
        MaterialPipelineDesc, MaterialType, Shader, ShaderSource,
    },
    surface::RenderSurface,
    RenderAssetExtractor, RenderAssets, RenderDevice,
};
use asset::{database::AssetDatabase, io::cache::LoadPath, plugin::AssetExt};
use ecs::{
    system::unlifetime::{ReadRes, WriteRes},
    world::action::WorldActions,
};
use game::{GameBuilder, Main, Plugin, SMain};

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
            .add_render_resource_extractor::<MaterialGlobals<M>>()
            .add_render_resource_extractor::<MaterialMesh<M>>()
            .add_render_resource_extractor::<M::Model>()
            .add_render_asset_dependency::<M, Shader>()
            .add_pipeline_extractor::<M>()
            .register_asset::<M>()
            .load_asset::<ShaderSource>(M::shader())
            .load_asset::<ShaderSource>(M::Pipeline::shader());

        let app = game.sub_app_mut::<RenderApp>();
        if !app.has_resource::<MaterialPipelines>() {
            app.add_resource(MaterialPipelines::new());
        }
    }
}

impl<M: Material> PipelineExtractor for M {
    type Arg = (
        ReadRes<RenderSurface>,
        ReadRes<MaterialGlobals<M>>,
        ReadRes<MaterialMesh<M>>,
        ReadRes<M::Model>,
        WriteRes<MaterialPipelines>,
        SMain<ReadRes<AssetDatabase>>,
    );

    fn kind() -> crate::resource::extract::PipelineExtractorKind {
        crate::resource::extract::PipelineExtractorKind::Render {
            vertex_shader: M::Pipeline::shader().into(),
            fragment_shader: M::shader().into(),
        }
    }

    fn extract_pipeline(
        device: &RenderDevice,
        shaders: &RenderAssets<Shader>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (surface, globals, mesh, model, pipelines, database) = arg;

        let vertex_shader = match M::Pipeline::shader().into() {
            LoadPath::Id(id) => shaders.get(&id.into()).unwrap(),
            LoadPath::Path(path) => {
                let library = database.library().read_blocking();
                let id = library.get_id(&path).unwrap();
                shaders.get(&id.into()).unwrap()
            }
        };

        let fragment_shader = match M::shader().into() {
            LoadPath::Id(id) => shaders.get(&id.into()).unwrap(),
            LoadPath::Path(path) => {
                let library = database.library().read_blocking();
                let id = library.get_id(&path).unwrap();
                shaders.get(&id.into()).unwrap()
            }
        };

        let desc = MaterialPipelineDesc {
            format: surface.format(),
            depth_format: Some(surface.depth_format()),
            globals: globals.value(),
            mesh: mesh.value(),
            model: model.value(),
            vertex_shader,
            fragment_shader,
        };

        pipelines.create_pipeline::<M>(device, desc);
    }

    fn remove_pipeline(arg: &mut ecs::system::ArgItem<Self::Arg>) {
        let (_, _, _, _, pipelines, _) = arg;

        pipelines.remove(MaterialType::of::<M>());
    }
}

impl<M: Material> RenderAssetExtractor for M {
    type Source = M;
    type Asset = MaterialInstance;
    type Arg = (
        ReadRes<RenderDevice>,
        WriteRes<MaterialPipelines>,
        Main<'static, WorldActions>,
        M::Arg,
    );

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::ExtractError> {
        let (device, pipelines, events, material_arg) = arg;

        let ty = MaterialType::of::<M>();
        let layout = pipelines.create_layout::<M>(device, *id);

        let binding = match source.bind_group(device, &layout, material_arg) {
            Ok(binding) => binding,
            Err(error) => return Err(error.into()),
        };

        events.add(ExtractPipeline::<M>::new());

        let instance = MaterialInstance {
            ty,
            binding,
            mode: M::mode(),
            model: M::Model::model(),
        };

        Ok(instance)
    }

    fn remove(
        id: &asset::AssetId,
        assets: &mut crate::RenderAssets<Self::Asset>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (_, layouts, _, _) = arg;

        if let Some(instance) = assets.remove(id) {
            if layouts.remove_dependency(instance.ty, id) {
                layouts.remove(instance.ty);
            }
        }
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
