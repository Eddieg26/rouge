use self::{
    buffer::Buffer,
    mesh::{Mesh, MeshInfo},
    texture::{Texture, Texture2d, TextureInfo},
};
use super::core::{gpu::GpuInstance, vertex::BaseVertex};
use std::{collections::HashMap, rc::Rc};

pub use id::*;

pub mod buffer;
pub mod id;
pub mod mesh;
pub mod texture;

pub type BufferId = ResourceId;
pub type MeshId = ResourceId;
pub type TextureId = ResourceId;

pub struct GraphicsResources {
    gpu: Rc<GpuInstance>,
    textures: HashMap<TextureId, Box<dyn Texture>>,
    buffers: HashMap<BufferId, Buffer>,
    meshes: HashMap<MeshId, Mesh>,
}

impl GraphicsResources {
    pub fn new(gpu: Rc<GpuInstance>) -> GraphicsResources {
        GraphicsResources {
            gpu,
            textures: HashMap::new(),
            buffers: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    pub fn gpu(&self) -> &GpuInstance {
        &self.gpu
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
        usage: wgpu::BufferUsages,
        id: &BufferId,
        data: T,
    ) -> Option<&Buffer> {
        let buffer = Buffer::from_data::<T>(self.gpu.device(), usage, &data);
        self.buffers.insert(*id, buffer);

        self.buffers.get(id)
    }

    pub fn create_texture(&mut self, id: &TextureId, info: &TextureInfo) -> Option<&dyn Texture> {
        let texture: Box<dyn Texture> = match info.dimension {
            wgpu::TextureDimension::D1 => todo!(),
            wgpu::TextureDimension::D2 => Box::new(Texture2d::from_info(
                self.gpu.device(),
                self.gpu.queue(),
                &info,
            )),
            wgpu::TextureDimension::D3 => todo!(),
        };

        self.textures.insert(*id, texture);

        self.dyn_texture(id)
    }

    pub fn create_mesh<T: BaseVertex>(
        &mut self,
        id: &MeshId,
        info: &MeshInfo<'_, T>,
    ) -> Option<&Mesh> {
        let mesh = Mesh::from_data(
            self.gpu.device(),
            info.vertices,
            info.indices,
            info.submeshes.to_vec(),
        );
        self.meshes.insert(*id, mesh);

        self.meshes.get(id)
    }
}
