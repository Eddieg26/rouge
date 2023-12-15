use self::{
    buffer::Buffer,
    mesh::{Mesh, MeshInfo},
    texture::{Texture, Texture2d, TextureInfo},
};
use super::core::{device::RenderDevice, vertex::BaseVertex};
use crate::ecs::{resource::ResourceId, Resource};
use std::collections::HashMap;

pub mod buffer;
pub mod material;
pub mod mesh;
pub mod shader;
pub mod texture;

pub type BufferId = ResourceId;
pub type MeshId = ResourceId;
pub type TextureId = ResourceId;
pub type MaterialId = ResourceId;

pub struct GraphicsResources {
    textures: HashMap<TextureId, Box<dyn Texture>>,
    buffers: HashMap<BufferId, Buffer>,
    meshes: HashMap<MeshId, Mesh>,
}

impl GraphicsResources {
    pub fn new() -> GraphicsResources {
        GraphicsResources {
            textures: HashMap::new(),
            buffers: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    pub fn texture<T: Texture>(&self, id: &TextureId) -> Option<&T> {
        self.textures
            .get(id)
            .and_then(|t| t.as_any().downcast_ref::<T>())
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.textures.get(id).and_then(|t| Some(t.as_ref()))
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    pub fn mesh(&self, id: &MeshId) -> Option<&Mesh> {
        self.meshes.get(id)
    }

    pub fn add_texture<T: Texture>(&mut self, id: &TextureId, texture: T) {
        self.textures.insert(*id, Box::new(texture));
    }

    pub fn add_buffer(&mut self, id: &BufferId, buffer: Buffer) {
        self.buffers.insert(*id, buffer);
    }

    pub fn add_mesh(&mut self, id: &MeshId, mesh: Mesh) {
        self.meshes.insert(*id, mesh);
    }

    pub fn create_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
        device: &RenderDevice,
        usage: wgpu::BufferUsages,
        id: &BufferId,
        data: T,
    ) -> Option<&Buffer> {
        let buffer = Buffer::from_data::<T>(device.inner(), usage, &data);
        self.buffers.insert(*id, buffer);

        self.buffers.get(id)
    }

    pub fn create_texture(
        &mut self,
        device: &RenderDevice,
        id: &TextureId,
        info: &TextureInfo,
    ) -> Option<&dyn Texture> {
        let texture: Box<dyn Texture> = match info.dimension {
            wgpu::TextureDimension::D1 => todo!(),
            wgpu::TextureDimension::D2 => {
                Box::new(Texture2d::from_info(device.inner(), device.queue(), &info))
            }
            wgpu::TextureDimension::D3 => todo!(),
        };

        self.textures.insert(*id, texture);

        self.dyn_texture(id)
    }

    pub fn create_mesh<T: BaseVertex>(
        &mut self,
        device: &RenderDevice,
        id: &MeshId,
        info: &MeshInfo<'_, T>,
    ) -> Option<&Mesh> {
        let mesh = Mesh::from_data(
            device.inner(),
            info.vertices,
            info.indices,
            info.submeshes.to_vec(),
        );
        self.meshes.insert(*id, mesh);

        self.meshes.get(id)
    }
}

impl Resource for GraphicsResources {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
