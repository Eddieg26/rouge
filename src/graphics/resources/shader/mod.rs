use self::graph::{
    attribute::{BufferProperty, PropertyBlock},
    ShaderConstants,
};
use super::{
    material::{BlendMode, Material, ShaderModel},
    GpuResources,
};
use crate::graphics::core::vertex::{BaseVertex, Vertex};

pub mod graph;

pub trait ShaderPipeline: 'static {}

pub struct ShaderInfo<'a> {
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub module: wgpu::ShaderModule,
    pub global_layout: &'a wgpu::BindGroupLayout,
    pub object_layout: &'a wgpu::BindGroupLayout,
    pub properties: &'a PropertyBlock,
}

pub struct Shader {
    model: ShaderModel,
    mode: BlendMode,
    module: wgpu::ShaderModule,
    material_layout: MaterialLayout,
    pipeline_layout: wgpu::PipelineLayout,
}

impl Shader {
    pub fn new(device: &wgpu::Device, info: ShaderInfo) -> Shader {
        let ShaderInfo {
            module,
            mode,
            model,
            global_layout,
            object_layout,
            properties,
        } = info;

        let inputs = properties.inputs();
        let padded = BufferProperty::add_padding(inputs);
        let buffer_size = padded.iter().map(|a| a.input().size()).sum::<u32>();
        let mut entries = vec![];
        if !inputs.is_empty() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: ShaderConstants::MATERIAL_BINDING as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(buffer_size as u64),
                },
                count: None,
            });
        }

        let textures = properties.textures();
        for texture in textures {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: entries.len() as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: texture.dimension(),
                },
                count: None,
            });
        }

        for _ in textures {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: entries.len() as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
            entries: &entries,
        });

        let mut layouts = vec![];
        layouts.insert(ShaderConstants::GLOBAL_BIND_GROUP, global_layout);
        layouts.insert(ShaderConstants::OBJECT_BIND_GROUP, object_layout);
        layouts.insert(ShaderConstants::MATERIAL_BIND_GROUP, &material_layout);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Material Pipeline Layout"),
            bind_group_layouts: &layouts,
            push_constant_ranges: &[],
        });

        let material_layout = {
            let buffer = if padded.is_empty() {
                None
            } else {
                Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: buffer_size as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }))
            };
            MaterialLayout::new(material_layout, buffer, properties.clone())
        };

        Shader {
            module,
            material_layout,
            pipeline_layout,
            model,
            mode,
        }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn material_layout(&self) -> &MaterialLayout {
        &self.material_layout
    }

    pub fn pipeline_layout(&self) -> &wgpu::PipelineLayout {
        &self.pipeline_layout
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn mode(&self) -> BlendMode {
        self.mode
    }

    // pub fn create_bind_group(
    //     &self,
    //     device: &wgpu::Device,
    //     resources: &GpuResources,
    //     material: &Material,
    // ) -> Option<wgpu::BindGroup> {
    //     let mut entries = vec![];

    //     if let Some(buffer) = &self.material_layout.buffer() {
    //         entries.push(wgpu::BindGroupEntry {
    //             binding: ShaderConstants::MATERIAL_BINDING as u32,
    //             resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
    //                 buffer,
    //                 offset: 0,
    //                 size: None,
    //             }),
    //         });
    //     }

    //     let textures = material.textures();
    //     for property in textures {
    //         if let Some(texture) = resources.texture_view(&property.texture()) {
    //             entries.push(wgpu::BindGroupEntry {
    //                 binding: entries.len() as u32,
    //                 resource: wgpu::BindingResource::TextureView(&texture),
    //             });
    //         }
    //     }

    //     for property in textures {
    //         if let Some(sampler) = resources.sampler(&property.texture()) {
    //             entries.push(wgpu::BindGroupEntry {
    //                 binding: entries.len() as u32,
    //                 resource: wgpu::BindingResource::Sampler(&sampler),
    //             });
    //         }
    //     }

    //     Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
    //         label: Some("Material Bind Group"),
    //         layout: self.material_layout.layout(),
    //         entries: &entries,
    //     }))
    // }

    pub fn create_pipeline(
        &self,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_write: DepthWrite,
    ) -> wgpu::RenderPipeline {
        let vertex = wgpu::VertexState {
            module: &self.module,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &Vertex::attributes(),
            }],
        };

        let depth_stencil = Some(wgpu::DepthStencilState {
            format: depth_write.format,
            depth_write_enabled: depth_write.enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&self.pipeline_layout),
            vertex,
            fragment: Some(wgpu::FragmentState {
                module: &self.module,
                entry_point: "fs_main",
                targets: &[Some(self.mode.color_target_state(color_format))],
            }),
            depth_stencil,
            primitive: wgpu::PrimitiveState::default(),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }
}

pub struct MaterialLayout {
    layout: wgpu::BindGroupLayout,
    buffer: Option<wgpu::Buffer>,
    properties: PropertyBlock,
}

impl MaterialLayout {
    pub fn new(
        layout: wgpu::BindGroupLayout,
        buffer: Option<wgpu::Buffer>,
        properties: PropertyBlock,
    ) -> MaterialLayout {
        MaterialLayout {
            layout,
            buffer,
            properties,
        }
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn buffer(&self) -> &Option<wgpu::Buffer> {
        &self.buffer
    }

    pub fn properties(&self) -> &PropertyBlock {
        &self.properties
    }
}

pub struct DepthWrite {
    pub enabled: bool,
    pub format: wgpu::TextureFormat,
}
