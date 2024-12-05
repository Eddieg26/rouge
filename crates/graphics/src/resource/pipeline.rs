use super::{binding::BindGroupLayout, mesh::MeshAttributeKind, shader::Shader, AtomicId, Handle};
use crate::{
    wgpu::{ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState, VertexStepMode},
    RenderAssets, RenderDevice,
};
use std::{borrow::Cow, num::NonZeroU32};

pub type RenderPipelineId = AtomicId<RenderPipeline>;

pub struct RenderPipeline {
    id: RenderPipelineId,
    inner: wgpu::RenderPipeline,
    instances: Option<NonZeroU32>,
}

impl std::ops::Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RenderPipeline {
    pub fn create(device: &RenderDevice, desc: RenderPipelineDesc) -> Self {
        let layout = desc.layout.map(|layout| {
            let layout = layout
                .iter()
                .map(|layout| layout.inner())
                .collect::<Vec<_>>();
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: layout.as_slice(),
                push_constant_ranges: &[],
            })
        });

        let vertex_shader = &desc.vertex.shader;

        let vertex_buffer_layouts = desc
            .vertex
            .buffers
            .iter()
            .map(|layout| wgpu::VertexBufferLayout {
                array_stride: layout.array_stride,
                step_mode: layout.step_mode,
                attributes: &layout.attributes,
            })
            .collect::<Vec<_>>();

        let vertex = wgpu::VertexState {
            module: vertex_shader.module(),
            entry_point: Some(&desc.vertex.entry),
            compilation_options: Default::default(),
            buffers: &vertex_buffer_layouts,
        };

        let fragment = match &desc.fragment {
            Some(state) => Some(wgpu::FragmentState {
                module: &state.shader,
                entry_point: Some(&state.entry),
                compilation_options: Default::default(),
                targets: &state.targets,
            }),
            None => None,
        };

        let instances = vertex_shader.meta().and_then(|m| m.instances());

        let desc = wgpu::RenderPipelineDescriptor {
            label: desc.label,
            layout: layout.as_ref(),
            vertex,
            primitive: desc.primitive,
            depth_stencil: desc.depth_state,
            fragment,
            multisample: desc.multisample,
            multiview: None,
            cache: None,
        };

        RenderPipeline {
            inner: device.create_render_pipeline(&desc),
            id: RenderPipelineId::new(),
            instances,
        }
    }

    pub fn with_instances(mut self, instances: NonZeroU32) -> Self {
        self.instances = Some(instances);
        self
    }

    pub fn id(&self) -> RenderPipelineId {
        self.id
    }

    pub fn inner(&self) -> &wgpu::RenderPipeline {
        &self.inner
    }

    pub fn instances(&self) -> Option<NonZeroU32> {
        self.instances
    }
}

impl From<wgpu::RenderPipeline> for RenderPipeline {
    fn from(pipeline: wgpu::RenderPipeline) -> Self {
        Self {
            inner: pipeline,
            id: RenderPipelineId::new(),
            instances: None,
        }
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct VertexBufferLayout {
    pub array_stride: u64,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn from(step_mode: VertexStepMode, layout: &[MeshAttributeKind]) -> Vec<Self> {
        layout
            .iter()
            .enumerate()
            .map(|(location, attribute)| Self {
                array_stride: attribute.size() as u64,
                step_mode,
                attributes: vec![wgpu::VertexAttribute {
                    format: attribute.format(),
                    offset: 0,
                    shader_location: location as u32,
                }],
            })
            .collect()
    }
}

pub struct VertexState<'a> {
    pub shader: &'a Shader,
    pub entry: &'a str,
    pub buffers: Vec<VertexBufferLayout>,
}

pub struct FragmentState<'a> {
    pub shader: &'a Shader,
    pub entry: &'a str,
    pub targets: Vec<Option<ColorTargetState>>,
}

pub struct RenderPipelineDesc<'a> {
    pub label: Option<&'a str>,
    pub layout: Option<&'a [&'a BindGroupLayout]>,
    pub vertex: VertexState<'a>,
    pub fragment: Option<FragmentState<'a>>,
    pub primitive: PrimitiveState,
    pub depth_state: Option<DepthStencilState>,
    pub multisample: MultisampleState,
}

pub struct ComputePipelineDesc<'a> {
    pub label: Option<&'a str>,
    pub layout: Option<&'a [&'a BindGroupLayout]>,
    pub shader: Handle<Shader>,
    pub entry: Cow<'static, str>,
}

pub struct ComputePipeline(wgpu::ComputePipeline);

impl ComputePipeline {
    pub fn create(
        device: &RenderDevice,
        desc: ComputePipelineDesc,
        shaders: &RenderAssets<Shader>,
    ) -> Option<Self> {
        let layout = desc.layout.map(|layout| {
            let layout = layout
                .iter()
                .map(|layout| layout.inner())
                .collect::<Vec<_>>();
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: layout.as_slice(),
                push_constant_ranges: &[],
            })
        });

        let shader = match &desc.shader {
            Handle::Ref(id) => shaders.get(id)?,
            Handle::Owned(shader) => shader,
        };

        let desc = wgpu::ComputePipelineDescriptor {
            label: desc.label,
            layout: layout.as_ref(),
            module: shader.module(),
            entry_point: Some(&desc.entry),
            compilation_options: Default::default(),
            cache: None,
        };

        Some(ComputePipeline(device.create_compute_pipeline(&desc)))
    }
}

impl std::ops::Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<wgpu::ComputePipeline> for ComputePipeline {
    fn from(pipeline: wgpu::ComputePipeline) -> Self {
        Self(pipeline)
    }
}

pub mod extract {
    use crate::{resource::Shader, RenderAssets, RenderDevice};
    use asset::io::cache::LoadPath;
    use ecs::{
        core::{resource::Resource, IndexMap, Type},
        system::{ArgItem, SystemArg},
        world::{
            action::{WorldAction, WorldActionFn},
            id::WorldKind,
            World,
        },
    };
    use std::collections::{hash_map::Entry, HashMap};
    use wgpu::ShaderStages;

    pub trait PipelineExtractor: Send + 'static {
        type Arg: SystemArg;

        fn kind() -> PipelineExtractorKind;
        fn extract_pipeline(
            device: &RenderDevice,
            shaders: &RenderAssets<Shader>,
            arg: &mut ArgItem<Self::Arg>,
        );
        fn remove_pipeline(arg: &mut ArgItem<Self::Arg>);
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub enum PipelineExtractorKind {
        Render {
            vertex_shader: LoadPath,
            fragment_shader: LoadPath,
        },
        Compute {
            shader: LoadPath,
        },
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum PipelineAction {
        Extract,
        Remove,
    }

    pub struct PipelineExtractorState {
        shaders: ShaderStages,
        shaders_loaded: ShaderStages,
        extract: fn() -> WorldActionFn,
        remove: fn() -> WorldActionFn,
        action: Option<PipelineAction>,
    }

    impl PipelineExtractorState {
        pub fn new(
            kind: &PipelineExtractorKind,
            extract: fn() -> WorldActionFn,
            remove: fn() -> WorldActionFn,
        ) -> Self {
            let shaders = match kind {
                PipelineExtractorKind::Render { .. } => {
                    ShaderStages::VERTEX | ShaderStages::FRAGMENT
                }
                PipelineExtractorKind::Compute { .. } => ShaderStages::COMPUTE,
            };

            Self {
                shaders,
                extract,
                remove,
                shaders_loaded: ShaderStages::empty(),
                action: None,
            }
        }

        pub fn shader_updated(&mut self, stage: ShaderStages, loaded: bool) {
            self.shaders_loaded.set(stage, loaded);
        }

        pub fn action(&self) -> Option<PipelineAction> {
            self.action
        }

        pub fn are_shaders_loaded(&self) -> bool {
            self.shaders.difference(self.shaders_loaded) == ShaderStages::empty()
        }

        pub fn extract(&self) -> WorldActionFn {
            (self.extract)()
        }

        pub fn remove(&self) -> WorldActionFn {
            (self.remove)()
        }
    }

    pub struct PipelineExtractors {
        extractors: IndexMap<Type, PipelineExtractorState>,
        shaders: HashMap<LoadPath, (Vec<usize>, ShaderStages)>,
        pub(crate) actions: IndexMap<Type, WorldActionFn>,
    }

    impl PipelineExtractors {
        pub fn new() -> Self {
            Self {
                extractors: IndexMap::new(),
                shaders: HashMap::new(),
                actions: IndexMap::new(),
            }
        }

        pub fn add_extractor<P: PipelineExtractor>(&mut self) {
            let ty = Type::of::<P>();
            if self.extractors.contains_key(&ty) {
                return;
            }

            let kind = P::kind();
            let extract = || ExtractPipeline::<P>::new().into();
            let remove = || RemovePipeline::<P>::new().into();
            let state = PipelineExtractorState::new(&kind, extract, remove);

            let index = self.extractors.len();
            self.extractors.insert(ty, state);

            match kind {
                PipelineExtractorKind::Render {
                    vertex_shader,
                    fragment_shader,
                } => {
                    self.add_shader(vertex_shader, ShaderStages::VERTEX, index);
                    self.add_shader(fragment_shader, ShaderStages::FRAGMENT, index);
                }
                PipelineExtractorKind::Compute { shader } => {
                    self.add_shader(shader, ShaderStages::COMPUTE, index);
                }
            }
        }

        pub fn extract<P: PipelineExtractor>(&mut self) {
            let ty = Type::of::<P>();
            if let Some(state) = self.extractors.get_mut(&ty) {
                state.action = Some(PipelineAction::Extract);
                if state.are_shaders_loaded() {
                    self.actions.insert(ty, state.extract());
                }
            }
        }

        pub fn remove<P: PipelineExtractor>(&mut self) {
            let ty = Type::of::<P>();
            if let Some(state) = self.extractors.get_mut(&ty) {
                if state.action == Some(PipelineAction::Extract) {
                    state.action = Some(PipelineAction::Remove);
                    self.actions.insert(ty, state.remove());
                }
            }
        }

        pub fn shader_updated(&mut self, path: LoadPath, loaded: bool) {
            if let Some((indices, stages)) = self.shaders.get(&path) {
                let mut actions = Vec::new();
                for index in indices {
                    let (extract, extractor) = {
                        let state = &mut self.extractors[*index];
                        state.shader_updated(*stages, loaded);

                        let extract = state.action == Some(PipelineAction::Extract)
                            && state.are_shaders_loaded();

                        (extract, state.extract())
                    };

                    if extract {
                        let ty = self.extractors.keys()[*index];
                        actions.push((ty, extractor));
                    }
                }

                self.actions.extend(actions);
            }
        }

        fn add_shader(&mut self, path: LoadPath, stage: ShaderStages, index: usize) {
            match self.shaders.entry(path) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().0.push(index);
                    entry.get_mut().1 |= stage;
                }
                Entry::Vacant(entry) => {
                    entry.insert((vec![index], stage));
                }
            }
        }
    }

    impl Resource for PipelineExtractors {}

    pub struct ExtractPipeline<P: PipelineExtractor>(std::marker::PhantomData<P>);

    impl<P: PipelineExtractor> ExtractPipeline<P> {
        pub fn new() -> Self {
            Self(Default::default())
        }
    }

    impl<P: PipelineExtractor> WorldAction for ExtractPipeline<P> {
        fn execute(self, world: &mut World) -> Option<()> {
            match world.kind() {
                WorldKind::Main => Some(world.resource_mut::<PipelineExtractors>().extract::<P>()),
                WorldKind::Sub => {
                    let world = unsafe { world.cell() };
                    let device = world.resource::<RenderDevice>();
                    let shaders = world.resource::<RenderAssets<Shader>>();
                    let mut arg = P::Arg::get(world);
                    Some(P::extract_pipeline(&device, &shaders, &mut arg))
                }
            }
        }
    }

    pub struct RemovePipeline<P: PipelineExtractor>(std::marker::PhantomData<P>);

    impl<P: PipelineExtractor> RemovePipeline<P> {
        pub fn new() -> Self {
            Self(Default::default())
        }
    }

    impl<P: PipelineExtractor> WorldAction for RemovePipeline<P> {
        fn execute(self, world: &mut World) -> Option<()> {
            match world.kind() {
                WorldKind::Main => Some(world.resource_mut::<PipelineExtractors>().remove::<P>()),
                WorldKind::Sub => {
                    let world = unsafe { world.cell() };
                    let mut arg = P::Arg::get(world);
                    Some(P::remove_pipeline(&mut arg))
                }
            }
        }
    }
}
