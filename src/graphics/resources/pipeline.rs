use super::{
    material::{BlendMode, Material, ShaderModel},
    shader::graph::attribute::PropertyBlock,
    GpuResources, ShaderId,
};
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthWrite {
    Auto,
    Enabled,
    Disabled,
}

pub trait PipelineConfig: 'static {
    fn depth_write(&self) -> DepthWrite;
    fn vertex(&self) -> wgpu::VertexState;
    fn primitive(&self) -> wgpu::PrimitiveState;
    fn color_format(&self) -> wgpu::TextureFormat;
    fn depth_format(&self) -> wgpu::TextureFormat;
    fn bind_group_layouts(&self) -> Vec<&wgpu::BindGroupLayout>;
}

pub trait MaterialPipeline: 'static {
    fn vertex_shader(&self) -> &ShaderId;
    fn fragment_shader(&self) -> &ShaderId;
    fn pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout;
}

pub struct MaterialShader {
    model: ShaderModel,
    blend_mode: BlendMode,
    properties: PropertyBlock,
    shader: wgpu::ShaderModule,
}

impl MaterialShader {
    pub fn new(
        model: ShaderModel,
        blend_mode: BlendMode,
        properties: PropertyBlock,
        shader: wgpu::ShaderModule,
    ) -> MaterialShader {
        MaterialShader {
            model,
            blend_mode,
            properties,
            shader,
        }
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    pub fn properties(&self) -> &PropertyBlock {
        &self.properties
    }

    pub fn shader(&self) -> &wgpu::ShaderModule {
        &self.shader
    }
}

pub struct MaterialPipelines {
    configs: HashMap<TypeId, Box<dyn PipelineConfig>>,
    pipelines: HashMap<TypeId, HashMap<ShaderId, wgpu::RenderPipeline>>,
}

impl MaterialPipelines {
    pub fn new() -> MaterialPipelines {
        MaterialPipelines {
            configs: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    pub fn add_config<T: PipelineConfig>(&mut self, config: T) {
        self.configs.insert(TypeId::of::<T>(), Box::new(config));
    }

    pub fn get<P: PipelineConfig>(&self, shader: &ShaderId) -> Option<&wgpu::RenderPipeline> {
        self.pipelines
            .get(&TypeId::of::<P>())
            .and_then(|p| p.get(shader))
    }

    pub fn create_pipelines<M: Material>(
        &mut self,
        device: &wgpu::Device,
        material: &M,
        resources: &GpuResources,
    ) {
        for (_, config) in &self.configs {
            let mut pipelines = HashMap::new();

            let fragment_shader = resources
                .shader(&material.fragment_shader())
                .expect("Shader not found");

            let material_layout = material.properties().create_bind_group_layout(device);

            let mut layouts = config.bind_group_layouts();
            layouts.push(&material_layout);

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Material Pipeline Layout"),
                bind_group_layouts: &layouts,
                push_constant_ranges: &[],
            });

            let color_format = config.color_format();
            let depth_format = config.depth_format();

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Material Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: config.vertex(),
                fragment: Some(wgpu::FragmentState {
                    entry_point: "main",
                    module: fragment_shader,
                    targets: &[Some(material.blend_mode().color_target_state(color_format))],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: match config.depth_write() {
                        DepthWrite::Auto => match material.blend_mode() {
                            BlendMode::Opaque => true,
                            BlendMode::Transparent => false,
                        },
                        DepthWrite::Enabled => true,
                        DepthWrite::Disabled => false,
                    },
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            pipelines.insert(material.fragment_shader(), pipeline);
        }
    }
}
