use itertools::Itertools;

use super::{material::Material, texture::Texture, GraphicsResources, MaterialId, PipelineId};
use std::collections::HashMap;

pub mod graph;
pub mod lit;
pub mod templates;
pub mod unlit;

pub trait ShaderTemplate {
    const MATERIAL_BIND_GROUP: u32;
    fn create_shader(device: &wgpu::Device, material: &Material) -> Shader;
}

pub struct Shader {
    entry: String,
    module: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    group_index: u32,
    buffer_size: u32,
    bind_groups: HashMap<MaterialId, wgpu::BindGroup>,
    pipelines: HashMap<PipelineId, wgpu::RenderPipeline>,
}

impl Shader {
    pub fn new(
        entry: &str,
        module: wgpu::ShaderModule,
        layout: wgpu::BindGroupLayout,
        group_index: u32,
        buffer_size: u32,
    ) -> Shader {
        Shader {
            entry: entry.to_string(),
            module,
            layout,
            group_index,
            buffer_size,
            bind_groups: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    fn contains(&self, id: &MaterialId) -> bool {
        self.bind_groups.contains_key(id)
    }

    pub fn pipeline(&self, id: &PipelineId) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(id)
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn buffer_size(&self) -> u32 {
        self.buffer_size
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn bind_group(&self, id: &MaterialId) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(id)
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: self.buffer_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_bind_group(
        &mut self,
        device: &wgpu::Device,
        resources: &GraphicsResources,
        id: &MaterialId,
        material: &Material,
    ) {
        if self.contains(id) {
            return;
        }

        let textures = Shader::get_textures(material, resources);
        let mut entries = vec![];

        let mut _buffer = None;
        if self.buffer_size > 0 {
            _buffer = Some(self.create_buffer(device));
            entries.push(wgpu::BindGroupEntry {
                binding: entries.len() as u32,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: _buffer.as_ref().unwrap(),
                    offset: 0,
                    size: None,
                }),
            });
        }

        for texture in &textures {
            entries.push(wgpu::BindGroupEntry {
                binding: (entries.len() + 1) as u32,
                resource: wgpu::BindingResource::TextureView(texture.view()),
            });
        }

        for texture in &textures {
            entries.push(wgpu::BindGroupEntry {
                binding: (entries.len() + 1) as u32,
                resource: wgpu::BindingResource::Sampler(texture.sampler()),
            });
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &entries,
            label: None,
        });

        self.bind_groups.insert(*id, bind_group);
    }

    pub fn create_pipeline(
        &mut self,
        device: &wgpu::Device,
        layouts: &[&wgpu::BindGroupLayout],
        info: &PipelineInfo,
    ) -> &wgpu::RenderPipeline {
        let mut layouts = layouts.iter().map(|l| *l).collect_vec();
        layouts.insert(self.group_index as usize, &self.layout);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &layouts,
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: info.vertex.clone(),
            fragment: Some(wgpu::FragmentState {
                module: &self.module,
                entry_point: &self.entry,
                targets: &info.targets,
            }),
            primitive: info.primitive,
            depth_stencil: info.depth_stencil.clone(),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        self.pipelines
            .entry(info.pipeline_id)
            .or_insert_with(|| pipeline)
    }

    fn get_textures<'a>(
        material: &Material,
        resources: &'a GraphicsResources,
    ) -> Vec<&'a dyn Texture> {
        material
            .textures()
            .iter()
            .filter_map(|id| resources.dyn_texture(id))
            .collect()
    }
}

pub struct PipelineInfo<'a> {
    pub pipeline_id: PipelineId,
    pub vertex: wgpu::VertexState<'a>,
    pub targets: Vec<Option<wgpu::ColorTargetState>>,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub primitive: wgpu::PrimitiveState,
}
