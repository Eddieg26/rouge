use super::{material::Material, texture::Texture, GraphicsResources, MaterialId};
use std::collections::HashMap;

pub mod templates;
pub mod unlit;

pub trait ShaderTemplate {
    fn create_shader(device: &wgpu::Device, material: &Material) -> Shader;
}

pub struct Shader {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    bind_groups: HashMap<MaterialId, wgpu::BindGroup>,
}

impl Shader {
    pub fn new(layout: wgpu::BindGroupLayout, pipeline: wgpu::RenderPipeline) -> Shader {
        Shader {
            pipeline,
            layout,
            bind_groups: HashMap::new(),
        }
    }

    fn contains(&self, id: &MaterialId) -> bool {
        self.bind_groups.contains_key(id)
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

    pub fn bind_group(&self, id: &MaterialId) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(id)
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
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
