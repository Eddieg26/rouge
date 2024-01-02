use super::buffer::Buffers;
use super::shader::layout::{
    AccessMode, BindingId, ShaderBinding, ShaderBindings, ShaderResource, ShaderResources,
    ShaderVariable,
};
use super::texture::{Texture, TextureResources};
use crate::ecs::resource::ResourceId;
use crate::ecs::Resource;
use std::collections::HashMap;

pub struct BindGroups {
    layouts: HashMap<BindingId, wgpu::BindGroupLayout>,
    bind_groups: HashMap<ResourceId, wgpu::BindGroup>,
}

impl BindGroups {
    pub fn new() -> BindGroups {
        BindGroups {
            layouts: HashMap::new(),
            bind_groups: HashMap::new(),
        }
    }

    pub fn layout(&self, id: &BindingId) -> Option<&wgpu::BindGroupLayout> {
        self.layouts.get(id)
    }

    pub fn create_bind_group(
        &mut self,
        device: &wgpu::Device,
        textures: &TextureResources,
        buffers: &mut Buffers,
        id: &ResourceId,
        resources: &ShaderResources,
    ) -> Option<&wgpu::BindGroup> {
        let mut entries = Vec::new();

        let bindings: ShaderBindings = resources
            .inner()
            .iter()
            .map(|p| Into::<ShaderBinding>::into(p.clone()))
            .collect::<Vec<_>>()
            .into();

        let binding_id = (&bindings).into();

        let layout = self.layouts.get(&binding_id)?;

        for resource in resources.inner() {
            let info = match resource {
                ShaderResource::Buffer(info) => info,
                _ => continue,
            };

            let inputs = info
                .inputs()
                .iter()
                .map(|i| Into::<ShaderVariable>::into(i.clone()))
                .collect::<Vec<_>>();

            buffers.register_buffer(device, &inputs);
        }

        for resource in resources.inner() {
            match resource {
                ShaderResource::Texture2D(texture) => {
                    let view = textures
                        .texture_2d(texture)
                        .map_or(&textures.defaults().texture_2d_view, |t| t.view());

                    entries.push(wgpu::BindGroupEntry {
                        binding: entries.len() as u32,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                ShaderResource::TextureCube(texture) => {
                    let view = textures
                        .texture_cube(texture)
                        .map_or(&textures.defaults().texture_cube_view, |t| t.view());

                    entries.push(wgpu::BindGroupEntry {
                        binding: entries.len() as u32,
                        resource: wgpu::BindingResource::TextureView(&view),
                    });
                }
                ShaderResource::Sampler(sampler) => {
                    let sampler = textures
                        .sampler(sampler)
                        .map_or(&textures.defaults().sampler, |s| s.inner());

                    entries.push(wgpu::BindGroupEntry {
                        binding: entries.len() as u32,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    });
                }
                ShaderResource::Buffer(info) => {
                    let id = info.into();
                    let buffer = buffers.get(&id).unwrap();

                    entries.push(wgpu::BindGroupEntry {
                        binding: entries.len() as u32,
                        resource: buffer.inner().as_entire_binding(),
                    });
                }
            }
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shader Bind Group"),
            layout,
            entries: &entries,
        });

        self.bind_groups.insert(id.clone(), bind_group);

        self.bind_groups.get(&id)
    }

    pub fn create_bind_group_layout(&mut self, device: &wgpu::Device, group: &ShaderBindings) {
        let id = group.into();

        if !self.layouts.contains_key(&id) {
            return;
        }

        let mut entries = Vec::new();

        for (slot, binding) in group.bindings().iter().enumerate() {
            let slot = slot as u32;
            match binding {
                ShaderBinding::UniformBuffer { count, .. } => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: slot,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: *count,
                    });
                }
                ShaderBinding::StorageBuffer { access, count, .. } => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: slot,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: *access == AccessMode::Read,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: *count,
                    });
                }
                ShaderBinding::Texture2D => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: slot,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    });
                }
                ShaderBinding::TextureCube => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: slot,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    });
                }
                ShaderBinding::Sampler => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: slot,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    });
                }
            }
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &entries,
        });

        self.layouts.insert(id, layout);
    }

    pub fn remove_bind_group(&mut self, id: &ResourceId) {
        self.bind_groups.remove(id);
    }
}

impl Resource for BindGroups {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
