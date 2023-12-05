use crate::{graphics::core::vertex::BaseVertex, primitives::bounds::Bounds};
use wgpu::util::DeviceExt;

pub struct MeshInfo<'a, T: BaseVertex> {
    pub vertices: &'a [T],
    pub indices: &'a [u32],
    pub submeshes: &'a [SubMesh],
}

impl<'a, T: BaseVertex> MeshInfo<'a, T> {
    pub fn new(vertices: &'a [T], indices: &'a [u32], submeshes: &'a [SubMesh]) -> MeshInfo<'a, T> {
        MeshInfo {
            vertices,
            indices,
            submeshes,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SubMesh {
    pub start_index: u32,
    pub index_count: u32,
}

pub struct Mesh {
    submeshes: Vec<SubMesh>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_count: u32,
    index_count: u32,
    bounds: Bounds,
}

impl Mesh {
    pub fn new(
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        vertex_count: u32,
        submeshes: Vec<SubMesh>,
        bounds: Bounds,
    ) -> Mesh {
        let index_count = submeshes.iter().map(|s| s.index_count).sum();

        Mesh {
            vertex_buffer,
            index_buffer,
            submeshes,
            vertex_count,
            index_count,
            bounds,
        }
    }

    pub fn from_data<T: BaseVertex>(
        device: &wgpu::Device,
        vertices: &[T],
        indices: &[u32],
        submeshes: Vec<SubMesh>,
    ) -> Mesh {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(vertices),
            label: None,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(indices),
            label: None,
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertex_count = vertices.len() as u32;
        let index_count = indices.len() as u32;
        let points = vertices.iter().map(|v| v.position()).collect::<Vec<_>>();

        Mesh {
            submeshes,
            vertex_buffer,
            index_buffer,
            vertex_count,
            index_count,
            bounds: Bounds::from_points(&points),
        }
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn submeshes(&self) -> &[SubMesh] {
        &self.submeshes
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }
}
