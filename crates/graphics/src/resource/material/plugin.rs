use super::{
    globals::{GlobalLayout, Globals},
    Material, MaterialInstance, MaterialLayouts, MaterialMetadata, MaterialPipelines,
    MeshPipelineData,
};
use crate::{
    plugin::{RenderApp, RenderAppExt, RenderPlugin},
    resource::{
        extract::PipelineExtractor,
        material::{MeshPipeline, Metadata},
        MaterialPipelineDesc, MaterialType, Shader, ShaderSource,
    },
    surface::RenderSurface,
    RenderAssetExtractor, RenderAssets, RenderDevice, View,
};
use asset::{database::AssetDatabase, io::cache::LoadPath, plugin::AssetExt};
use ecs::{
    event::{Event, Events},
    system::unlifetime::{ReadRes, WriteRes},
};
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
        if !game.has_resource::<MaterialPipelines>() {
            game.add_resource(MaterialPipelines::new());
        }

        game.add_render_asset_extractor::<M>()
            .add_render_resource_extractor::<GlobalLayout>()
            .add_render_resource_extractor::<MaterialMetadata<M::Meta>>()
            .add_render_resource_extractor::<MeshPipelineData<M::Pipeline>>()
            .add_render_asset_dependency::<M, Shader>()
            .add_pipeline_extractor::<M>()
            .register_asset::<M>()
            .load_asset::<ShaderSource>(M::shader())
            .load_asset::<ShaderSource>(M::Pipeline::shader());
    }

    fn finish(&mut self, game: &mut GameBuilder) {
        let app = game.sub_app_mut::<RenderApp>();
        if !app.has_resource::<MaterialLayouts>() {
            app.add_resource(MaterialLayouts::new());
        }

        app.register_event::<ModifyMaterialType<M>>();
    }
}

pub struct ModifyMaterialType<M: Material> {
    pub remove: bool,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> ModifyMaterialType<M> {
    pub fn new(remove: bool) -> Self {
        Self {
            remove,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: Material> Event for ModifyMaterialType<M> {}

impl<M: Material> PipelineExtractor for M {
    type Arg = (
        ReadRes<RenderSurface>,
        ReadRes<GlobalLayout>,
        ReadRes<MeshPipelineData<M::Pipeline>>,
        ReadRes<MaterialMetadata<M::Meta>>,
        ReadRes<MaterialLayouts>,
        WriteRes<MaterialPipelines>,
        Main<'static, ReadRes<AssetDatabase>>,
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
        let (surface, global_layout, surface_pipeline, metadata, layouts, pipelines, database) =
            arg;

        let ty = MaterialType::of::<M>();
        let layout = layouts.get(&ty).unwrap();

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
            global_layout: &global_layout,
            mesh: &surface_pipeline.data,
            metadata: &metadata.metadata,
            vertex_shader,
            fragment_shader,
        };

        println!(
            "Creating pipeline for material: {}",
            std::any::type_name::<M>()
        );
        pipelines.add::<M>(device, desc, layout);
    }

    fn remove_pipeline(arg: &mut ecs::system::ArgItem<Self::Arg>) {
        let (_, _, _, _, _, pipelines, _) = arg;

        pipelines.remove(&MaterialType::of::<M>());
    }
}

impl<M: Material> RenderAssetExtractor for M {
    type Source = M;
    type Asset = MaterialInstance;
    type Arg = (
        ReadRes<RenderDevice>,
        WriteRes<MaterialLayouts>,
        WriteRes<Events<ModifyMaterialType<M>>>,
        M::Arg,
    );

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::ExtractError> {
        let (device, layouts, events, material_arg) = arg;

        let ty = MaterialType::of::<M>();
        let layout = match layouts.get(&ty) {
            Some(layout) => layout.layout().clone(),
            None => layouts.add::<M>(device, *id),
        };

        let binding = match source.bind_group(device, &layout, material_arg) {
            Ok(binding) => binding,
            Err(error) => return Err(error.into()),
        };

        events.add(ModifyMaterialType::new(false));

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
        let (_, layouts, events, _) = arg;

        if let Some(instance) = assets.remove(id) {
            if layouts.remove_dependency(&instance.ty, id) {
                events.add(ModifyMaterialType::new(true));
            }
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
