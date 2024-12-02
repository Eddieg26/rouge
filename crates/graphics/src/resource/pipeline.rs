use super::{binding::BindGroupLayout, mesh::MeshAttributeKind, shader::Shader, AtomicId, Handle};
use crate::{
    wgpu::{ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState, VertexStepMode},
    RenderAssets, RenderDevice,
};
use asset::io::cache::LoadPath;
use ecs::{
    core::{resource::Resource, IndexMap, Type},
    event::Event,
    system::{ArgItem, SystemArg, SystemFunc},
    world::World,
};
use std::{borrow::Cow, collections::HashMap, num::NonZeroU32, sync::Arc};

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
    pub fn create<'a>(device: &RenderDevice, desc: RenderPipelineDesc<'a>) -> Self {
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

pub trait RenderPipelineExtractor: 'static {
    type Trigger: Event;
    type RemoveTrigger: Event;
    type Arg: SystemArg;

    fn vertex_shader() -> impl Into<LoadPath>;
    fn fragment_shader() -> impl Into<LoadPath>;

    fn create_pipeline(
        device: &RenderDevice,
        shaders: &RenderAssets<Shader>,
        arg: &mut ArgItem<Self::Arg>,
    );

    fn remove_pipeline(arg: &mut ArgItem<Self::Arg>);
}

pub struct RenderPipelineState {
    pub event_triggered: bool,
    pub vertex_shader_loaded: bool,
    pub fragment_shader_loaded: bool,
    pub create_pipeline: SystemFunc,
    pub remove_pipeline: SystemFunc,
}

pub struct RenderPipelineExtractors {
    extractors: IndexMap<Type, RenderPipelineState>,
    shader_map: HashMap<LoadPath, (Vec<usize>, bool)>,
}

impl RenderPipelineExtractors {
    pub fn new() -> Self {
        Self {
            extractors: IndexMap::new(),
            shader_map: HashMap::new(),
        }
    }

    pub fn get(&self, ty: Type) -> Option<&RenderPipelineState> {
        self.extractors.get(&ty)
    }

    pub fn get_mut(&mut self, ty: Type) -> Option<&mut RenderPipelineState> {
        self.extractors.get_mut(&ty)
    }

    pub fn add<E: RenderPipelineExtractor>(&mut self) {
        let ty = Type::of::<E>();
        if self.extractors.contains_key(&ty) {
            return;
        }

        let vs = E::vertex_shader().into();
        let fs = E::fragment_shader().into();

        let index = self.extractors.len();
        self.shader_map
            .entry(vs)
            .or_insert((vec![], true))
            .0
            .push(index);
        self.shader_map
            .entry(fs)
            .or_insert((vec![], false))
            .0
            .push(index);

        self.extractors.insert(
            ty,
            RenderPipelineState {
                event_triggered: false,
                vertex_shader_loaded: false,
                fragment_shader_loaded: false,
                create_pipeline: Arc::new(|world| {
                    let device = world.resource::<RenderDevice>();
                    let shaders = world.resource::<RenderAssets<Shader>>();
                    let mut arg = E::Arg::get(world);
                    E::create_pipeline(&device, &shaders, &mut arg);
                }),
                remove_pipeline: Arc::new(|world| {
                    let mut arg = E::Arg::get(world);
                    E::remove_pipeline(&mut arg);
                }),
            },
        );
    }

    pub fn remove<E: RenderPipelineExtractor>(&mut self, world: &World) {
        let world = unsafe { world.cell() };
        let ty = Type::of::<E>();
        if let Some(state) = self.extractors.get_mut(&ty) {
            (state.remove_pipeline)(world);
            state.event_triggered = false;
        }
    }

    pub fn shader_updated(&mut self, world: &World, path: LoadPath, loaded: bool) {
        let world = unsafe { world.cell() };
        if let Some((indices, is_vs)) = self.shader_map.get(&path) {
            for &index in indices {
                let state = &mut self.extractors[index];
                if *is_vs {
                    state.vertex_shader_loaded = loaded;
                } else {
                    state.fragment_shader_loaded = loaded;
                }

                if state.event_triggered
                    && state.vertex_shader_loaded
                    && state.fragment_shader_loaded
                {
                    (state.create_pipeline)(world);
                }
            }
        }
    }
}

impl Resource for RenderPipelineExtractors {}
