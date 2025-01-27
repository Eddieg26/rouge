use super::{
    asset::RenderAssets,
    resource::{RenderResourceExtractor, RenderResourceExtractors},
};
use crate::{core::RenderApp, resource::Shader, RenderDevice};
use asset::{database::AssetDatabase, io::cache::LoadPath};
use ecs::{
    core::resource::Resource,
    system::{unlifetime::ReadRes, ArgItem, SystemArg},
    world::{action::WorldAction, id::WorldKind, World},
};
use game::SubActions;

#[derive(Clone, Debug)]
pub enum PipelineShaders {
    Render {
        vertex: LoadPath,
        fragment: LoadPath,
    },
    Compute {
        shader: LoadPath,
    },
}

impl<L: Into<LoadPath>, R: Into<LoadPath>> From<(L, R)> for PipelineShaders {
    fn from((vertex, fragment): (L, R)) -> Self {
        Self::Render {
            vertex: vertex.into(),
            fragment: fragment.into(),
        }
    }
}

impl PipelineShaders {
    pub fn render(vertex: impl Into<LoadPath>, fragment: impl Into<LoadPath>) -> Self {
        Self::Render {
            vertex: vertex.into(),
            fragment: fragment.into(),
        }
    }

    pub fn compute(shader: impl Into<LoadPath>) -> Self {
        Self::Compute {
            shader: shader.into(),
        }
    }
}

pub trait PipelineExtractor: Resource + Send {
    type Arg: SystemArg;

    fn shaders() -> PipelineShaders;
    fn extract(
        device: &RenderDevice,
        shaders: &RenderAssets<Shader>,
        arg: ArgItem<Self::Arg>,
    ) -> Self;
}

impl<P: PipelineExtractor> RenderResourceExtractor for P {
    type Arg = (<P as PipelineExtractor>::Arg, ReadRes<RenderAssets<Shader>>);

    fn can_extract(world: &World) -> bool {
        let shaders = world.resource::<RenderAssets<Shader>>();
        let database = world.resource::<AssetDatabase>();
        match P::shaders() {
            PipelineShaders::Render { vertex, fragment } => {
                let vertex = match vertex {
                    LoadPath::Id(id) => shaders.contains(id.into()),
                    LoadPath::Path(path) => database
                        .path_id(&path)
                        .is_some_and(|id| shaders.contains(id.into())),
                };

                let fragment = match fragment {
                    LoadPath::Id(id) => shaders.contains(id.into()),
                    LoadPath::Path(path) => database
                        .path_id(&path)
                        .is_some_and(|id| shaders.contains(id.into())),
                };

                vertex && fragment
            }
            PipelineShaders::Compute { shader } => match shader {
                LoadPath::Id(id) => shaders.contains(id.into()),
                LoadPath::Path(path) => database
                    .path_id(&path)
                    .is_some_and(|id| shaders.contains(id.into())),
            },
        }
    }

    fn extract(device: &RenderDevice, arg: ArgItem<Self::Arg>) -> Self {
        let (arg, mut shaders) = arg;
        Self::extract(device, &mut shaders, arg)
    }
}

pub struct ExtractPipeline<P: PipelineExtractor> {
    _phantom: std::marker::PhantomData<P>,
}

impl<P: PipelineExtractor> ExtractPipeline<P> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: PipelineExtractor> WorldAction for ExtractPipeline<P> {
    fn execute(self, world: &mut World) -> Option<()> {
        match world.kind() {
            WorldKind::Main => {
                let actions = world.resource_mut::<SubActions<RenderApp>>();
                actions.add(ExtractPipeline::<P>::new());
            }
            WorldKind::Sub => {
                let extractors = world.resource_mut::<RenderResourceExtractors>();
                extractors.add::<P>();
            }
        }

        Some(())
    }
}
