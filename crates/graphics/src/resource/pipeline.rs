use super::{binding::BindGroupLayout, mesh::MeshAttributeKind, shader::Shader, AtomicId, Handle};
use crate::core::{RenderAssets, RenderDevice};
use std::{borrow::Cow, num::NonZeroU32};

pub use wgpu::{
    ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState, VertexStepMode,
};

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
    pub fn create(
        device: &RenderDevice,
        desc: RenderPipelineDesc,
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

        let vertex_shader = match &desc.vertex.shader {
            Handle::Ref(id) => shaders.get(id)?,
            Handle::Owned(shader) => shader,
        };

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
                module: match &state.shader {
                    Handle::Ref(id) => shaders.get(id)?.module(),
                    Handle::Owned(shader) => shader.module(),
                },
                entry_point: Some(&state.entry),
                compilation_options: Default::default(),
                targets: &state.targets,
            }),
            None => None,
        };

        let instances = desc.instances;

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

        Some(RenderPipeline {
            inner: device.create_render_pipeline(&desc),
            id: RenderPipelineId::new(),
            instances,
        })
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

pub struct VertexState {
    pub shader: Handle<Shader>,
    pub entry: Cow<'static, str>,
    pub buffers: Vec<VertexBufferLayout>,
}

pub struct FragmentState {
    pub shader: Handle<Shader>,
    pub entry: Cow<'static, str>,
    pub targets: Vec<Option<ColorTargetState>>,
}

pub struct RenderPipelineDesc<'a> {
    pub label: Option<&'a str>,
    pub layout: Option<&'a [&'a BindGroupLayout]>,
    pub vertex: VertexState,
    pub fragment: Option<FragmentState>,
    pub primitive: PrimitiveState,
    pub depth_state: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub instances: Option<NonZeroU32>,
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
