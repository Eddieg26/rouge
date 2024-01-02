use super::{bind_group::BindGroups, shader::Shader};
use crate::{
    asset::Assets,
    ecs::{resource::ResourceType, Resource},
    graphics::{core::BlendMode, resources::ShaderId},
};
use itertools::Itertools;
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthWrite {
    Auto,
    Enabled,
    Disabled,
}

pub trait ShaderPipeline: 'static {
    fn vertex() -> ShaderId;
    fn depth_write() -> DepthWrite;
    fn primitive() -> wgpu::PrimitiveState;
    fn color_format() -> wgpu::TextureFormat;
    fn depth_format() -> wgpu::TextureFormat;
}

pub struct PipelineConfig {
    vertex: ShaderId,
    depth_write: DepthWrite,
    primitive: wgpu::PrimitiveState,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
}

impl PipelineConfig {
    pub fn new<P: ShaderPipeline>() -> PipelineConfig {
        PipelineConfig {
            vertex: P::vertex(),
            depth_write: P::depth_write(),
            primitive: P::primitive(),
            color_format: P::color_format(),
            depth_format: P::depth_format(),
        }
    }

    pub fn vertex(&self) -> &ShaderId {
        &self.vertex
    }

    pub fn depth_write(&self) -> DepthWrite {
        self.depth_write
    }

    pub fn primitive(&self) -> &wgpu::PrimitiveState {
        &self.primitive
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    config: ResourceType,
    fragment: ShaderId,
}

impl PipelineKey {
    pub fn new(config: ResourceType, fragment: ShaderId) -> PipelineKey {
        PipelineKey { config, fragment }
    }

    pub fn config(&self) -> ResourceType {
        self.config
    }

    pub fn fragment(&self) -> ShaderId {
        self.fragment
    }
}

pub struct Pipelines {
    configs: HashMap<ResourceType, PipelineConfig>,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
}

impl Pipelines {
    pub fn new() -> Pipelines {
        Pipelines {
            configs: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    pub fn register<P: ShaderPipeline>(&mut self) {
        self.configs
            .insert(TypeId::of::<P>().into(), PipelineConfig::new::<P>());
    }

    pub fn config<P: ShaderPipeline>(&self) -> Option<&PipelineConfig> {
        self.configs.get(&TypeId::of::<P>().into())
    }

    pub fn pipeline<P: ShaderPipeline>(&self, fragment: ShaderId) -> Option<&wgpu::RenderPipeline> {
        let key = PipelineKey::new(TypeId::of::<P>().into(), fragment);
        self.pipelines.get(&key)
    }

    pub fn add_pipelines(
        &mut self,
        device: &wgpu::Device,
        fragment_id: ShaderId,
        blend_mode: BlendMode,
        shaders: &Assets<Shader>,
        bind_groups: &BindGroups,
    ) {
        for (config, pipeline) in self.configs.iter() {
            let key = PipelineKey::new(*config, fragment_id);

            if self.pipelines.contains_key(&key) {
                continue;
            }

            let vertex = shaders
                .get(pipeline.vertex())
                .expect("Missing vertex shader");

            let fragment = shaders.get(&fragment_id).expect("Missing fragment shader");

            if !vertex.validate(fragment) {
                panic!("Vertex shader output does not match fragment shader input");
            }

            let shaders = &[vertex, fragment];
            let layouts = Self::get_layouts(&bind_groups, shaders);

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &layouts,
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vertex.module(),
                    entry_point: vertex.meta().entry(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment.module(),
                    entry_point: fragment.meta().entry(),
                    targets: &[Some(blend_mode.color_target_state(pipeline.color_format()))],
                }),
                primitive: pipeline.primitive().clone(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: pipeline.depth_format(),
                    depth_write_enabled: match pipeline.depth_write() {
                        DepthWrite::Auto => match pipeline.primitive().topology {
                            wgpu::PrimitiveTopology::TriangleList
                            | wgpu::PrimitiveTopology::TriangleStrip => true,
                            _ => false,
                        },
                        DepthWrite::Enabled => true,
                        DepthWrite::Disabled => false,
                    },
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

            self.pipelines.insert(key, pipeline);
        }
    }

    fn get_layouts<'a>(
        bind_groups: &'a BindGroups,
        shaders: &'a [&Shader],
    ) -> Vec<&'a wgpu::BindGroupLayout> {
        let mut groups = HashMap::new();
        for shader in shaders {
            for binding in shader.meta().bindings() {
                if let Some(prev) = groups.insert(binding.group(), binding.clone()) {
                    if prev != *binding {
                        panic!("Mismatched binding layouts");
                    }
                }
            }
        }

        groups
            .values()
            .map(|binding| {
                bind_groups
                    .layout(&binding.into())
                    .expect("Missing bind group layout")
            })
            .collect_vec()
    }
}

impl Resource for Pipelines {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
