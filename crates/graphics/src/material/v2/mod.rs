use std::{collections::HashMap, hash::Hash};

use crate::{
    extract::RenderAssets,
    renderer::state::RenderState,
    resource::{
        BindGroup, BindGroupLayout, BufferData, Mesh, MeshAttributeKind, RenderBufferArray,
        RenderMesh, SubMesh, VertexBufferLayout,
    },
    Viewport,
};
use asset::{asset::Asset, io::cache::LoadPath, AssetRef};
use spatial::Aabb;
use wgpu::{BlendState, PrimitiveState};

pub trait View: BufferData + Send + Sync + 'static {
    type Data: Send + Sync + 'static;

    fn world(&self) -> glam::Mat4;
    fn view(&self) -> glam::Mat4;
    fn projection(&self) -> &glam::Mat4;

    fn position(&self) -> glam::Vec3 {
        let world = self.world();
        glam::Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }
}

pub struct RenderView<V: View> {
    entity: ecs::Entity,
    view: V,
    data: V::Data,
    depth: i32,
    viewport: Option<Viewport>,
}

impl<V: View> RenderView<V> {
    pub fn new(
        entity: ecs::Entity,
        view: V,
        depth: i32,
        data: V::Data,
        viewport: Option<Viewport>,
    ) -> Self {
        Self {
            entity,
            view,
            data,
            depth,
            viewport,
        }
    }

    pub fn entity(&self) -> ecs::Entity {
        self.entity
    }

    pub fn data(&self) -> &V::Data {
        &self.data
    }

    pub fn depth(&self) -> i32 {
        self.depth
    }

    pub fn viewport(&self) -> Option<&Viewport> {
        self.viewport.as_ref()
    }
}

impl<V: View> std::ops::Deref for RenderView<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<V: View> std::ops::DerefMut for RenderView<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
    }
}

pub struct RenderViews<V: View> {
    views: Vec<RenderView<V>>,
}

impl<V: View> RenderViews<V> {
    pub fn new() -> Self {
        Self { views: Vec::new() }
    }

    pub fn add(&mut self, view: RenderView<V>) {
        self.views.push(view);
    }

    pub fn get(&self, index: usize) -> Option<&RenderView<V>> {
        self.views.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut RenderView<V>> {
        self.views.get_mut(index)
    }

    pub fn views(&self) -> &[RenderView<V>] {
        &self.views
    }
}

pub trait MeshData: BufferData + Copy + Send + Sync + 'static {
    fn world(&self) -> glam::Mat4;
    fn position(&self) -> glam::Vec3 {
        let world = self.world();
        glam::Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }
}

pub struct MeshDataBuffer<M: MeshData> {
    inner: RenderBufferArray<M>,
    layout: Option<BindGroupLayout>,
    binding: Option<BindGroup>,
}

pub trait MeshRenderer {
    type View: View;
    type Mesh: MeshData;

    fn primitive() -> PrimitiveState;
    fn vertex_attributes() -> Vec<VertexAttribute>;
    fn instance_attributes() -> Vec<VertexAttribute> {
        vec![]
    }
    fn depth_write() -> DepthWrite;
    fn shader() -> impl Into<LoadPath>;
}

pub trait Material: Asset + Send + Sync + 'static {
    type Renderer: MeshRenderer;

    fn mode() -> BlendMode;
    fn shader() -> impl Into<LoadPath>;
}

pub type MaterialMeshData<M> = <<M as Material>::Renderer as MeshRenderer>::Mesh;
pub type MaterialView<M> = <<M as Material>::Renderer as MeshRenderer>::View;

pub trait Draw: Send + Sync + 'static {
    type Material: Material;

    const BATCH: bool = true;
    const CULL: bool = false;
    const SORT: bool = false;

    fn entity(&self) -> ecs::Entity;
    fn material(&self) -> AssetRef<Self::Material>;
    fn mesh(&self) -> AssetRef<Mesh>;
    fn submesh(&self) -> Option<SubMesh> {
        None
    }
    fn bounds(&self) -> Aabb {
        Aabb::MAX
    }
    fn data(&self) -> MaterialMeshData<Self::Material>;
}

pub struct DrawCall<D: Draw> {
    draw: D,
    bounds: Aabb,
    data: MaterialMeshData<D::Material>,
}

impl<D: Draw> DrawCall<D> {
    pub fn new(draw: D) -> Self {
        let bounds = draw.bounds();
        let data = draw.data();

        Self { draw, bounds, data }
    }

    pub fn draw(&self) -> &D {
        &self.draw
    }

    pub fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub fn data(&self) -> &MaterialMeshData<D::Material> {
        &self.data
    }
}

pub struct BatchKey<M: Material> {
    pub material: AssetRef<M>,
    pub mesh: AssetRef<Mesh>,
    pub sub_mesh: Option<SubMesh>,
}

impl<M: Material> BatchKey<M> {
    pub fn new(material: AssetRef<M>, mesh: AssetRef<Mesh>, sub_mesh: Option<SubMesh>) -> Self {
        BatchKey {
            material,
            mesh,
            sub_mesh,
        }
    }
}

impl<M: Material> Eq for BatchKey<M> {}
impl<M: Material> PartialEq for BatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material
            && self.mesh == other.mesh
            && self.sub_mesh == other.sub_mesh
    }
}

impl<M: Material> Hash for BatchKey<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.material.hash(state);
        self.mesh.hash(state);
        self.sub_mesh.hash(state);
    }
}

impl<M: Material> Copy for BatchKey<M> {}
impl<M: Material> Clone for BatchKey<M> {
    fn clone(&self) -> Self {
        BatchKey {
            material: self.material.clone(),
            mesh: self.mesh.clone(),
            sub_mesh: self.sub_mesh,
        }
    }
}

pub struct DrawCalls<D: Draw> {
    calls: Vec<DrawCall<D>>,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Batch {
    index: u32,
    size: u32,
}

impl Batch {
    pub fn new(index: u32, size: u32) -> Self {
        Self { index, size }
    }
}

pub struct BatchedDrawCalls<D: Draw> {
    batches: HashMap<BatchKey<D::Material>, Batch>,
    unbatched: Vec<(usize, u32)>,
}

impl<D: Draw> BatchedDrawCalls<D> {
    pub fn new() -> Self {
        Self {
            batches: HashMap::new(),
            unbatched: Vec::new(),
        }
    }
}

pub struct ViewDrawCalls<D: Draw> {
    views: HashMap<ecs::Entity, BatchedDrawCalls<D>>,
}

fn prepare_view_draw_calls<D: Draw>(
    views: &RenderViews<MaterialView<D::Material>>,
    draw_calls: &DrawCalls<D>,
    view_draw_calls: &mut ViewDrawCalls<D>,
    buffer: &mut MeshDataBuffer<MaterialMeshData<D::Material>>,
) {
    for view in views.views() {
        let mut batched_calls: HashMap<BatchKey<D::Material>, Vec<usize>> = HashMap::new();
        let mut unbatched_calls: Vec<usize> = Vec::new();

        for (index, call) in draw_calls.calls.iter().enumerate() {
            if D::BATCH {
                let key =
                    BatchKey::new(call.draw.material(), call.draw.mesh(), call.draw.submesh());

                batched_calls
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(index);
            } else {
                unbatched_calls.push(index);
            }
        }

        let mut view_calls = BatchedDrawCalls::<D>::new();

        for (key, indices) in batched_calls {
            let mut batch = Batch::new(u32::MAX, indices.len() as u32);

            for index in indices {
                let call = &draw_calls.calls[index];
                let mesh_data = call.data();
                let buffer_index = buffer.inner.push(*mesh_data);
                batch.index = buffer_index.index.min(batch.index);
            }

            view_calls.batches.insert(key, batch);
        }

        if D::SORT {
            let view_pos = view.position();
            unbatched_calls.sort_by(|a, b| {
                let a = &draw_calls.calls[*a];
                let b = &draw_calls.calls[*b];

                let a_pos = a.data.position() - view_pos;
                let b_pos = b.data.position() - view_pos;

                a_pos
                    .length()
                    .partial_cmp(&b_pos.length())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        for index in unbatched_calls {
            let call = &draw_calls.calls[index];
            let mesh_data = call.data();
            let buffer_index = buffer.inner.push(*mesh_data);
            view_calls.unbatched.push((index, buffer_index.index));
        }

        view_draw_calls.views.insert(view.entity(), view_calls);
    }
}

fn render_draw_calls<D: Draw>(
    view_draw_calls: &ViewDrawCalls<D>,
    draw_calls: &DrawCalls<D>,
    buffer: &MeshDataBuffer<MaterialMeshData<D::Material>>,
    meshes: &RenderAssets<RenderMesh>,
    state: &mut RenderState,
) {
    let Some(mesh_buffer) = buffer.inner.buffer() else {
        return;
    };

    let attributes = <D::Material as Material>::Renderer::vertex_attributes();
    state.set_vertex_buffer(attributes.len() as u32, mesh_buffer.slice(..));

    let mut render = |mesh: AssetRef<Mesh>, sub_mesh: Option<SubMesh>, batch: Batch| {
        let mesh = match meshes.get(&mesh) {
            Some(mesh) => mesh,
            None => return,
        };

        let sub_mesh = match sub_mesh {
            Some(sub_mesh) => sub_mesh,
            None => SubMesh::from(mesh),
        };

        match mesh.vertex_buffers_by_attributes(&[]) {
            Some(vertex_buffers) => vertex_buffers.iter().for_each(|buffer| {
                let slice = buffer
                    .slice(sub_mesh.start_vertex..sub_mesh.start_vertex + sub_mesh.vertex_count);

                state.set_vertex_buffer(0, slice);
            }),
            None => return,
        };

        match mesh.index_buffer() {
            Some(index_buffer) => {
                let index_buffer = index_buffer
                    .slice(sub_mesh.start_index..sub_mesh.start_index + sub_mesh.index_count);
                let instances = batch.index..batch.index + batch.size;

                state.set_index_buffer(index_buffer);
                state.draw_indexed(0..sub_mesh.index_count as u32, 0, instances);
            }
            None => {
                let instances = batch.index..batch.index + batch.size;
                state.draw(0..sub_mesh.vertex_count as u32, instances);
            }
        }
    };

    for (view, visible_calls) in view_draw_calls.views.iter() {
        for (key, batch) in &visible_calls.batches {
            let mesh = key.mesh;
            let sub_mesh = key.sub_mesh;

            render(mesh, sub_mesh, *batch);
        }

        for (index, offset) in &visible_calls.unbatched {
            let call = &draw_calls.calls[*index];
            let mesh = call.draw.mesh();
            let sub_mesh = call.draw.submesh();

            render(mesh, sub_mesh, Batch::new(*offset, 1));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl Into<BlendState> for BlendMode {
    fn into(self) -> wgpu::BlendState {
        match self {
            Self::Opaque => wgpu::BlendState::REPLACE,
            Self::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    Auto,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexAttribute {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,
}

impl VertexAttribute {
    pub fn size(&self) -> u64 {
        match self {
            Self::Float => 4,
            Self::Vec2 => 8,
            Self::Vec3 => 12,
            Self::Vec4 => 16,
            Self::Color => 4,
        }
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            Self::Float => wgpu::VertexFormat::Float32,
            Self::Vec2 => wgpu::VertexFormat::Float32x2,
            Self::Vec3 => wgpu::VertexFormat::Float32x3,
            Self::Vec4 => wgpu::VertexFormat::Float32x4,
            Self::Color => wgpu::VertexFormat::Float32x4,
        }
    }

    pub fn into_layout(attributes: &[Self], mode: wgpu::VertexStepMode) -> VertexBufferLayout {
        let mut stride = 0;
        let attributes = attributes
            .iter()
            .enumerate()
            .map(|(location, a)| {
                let format = a.format();
                let offset = stride;
                stride += a.size();
                wgpu::VertexAttribute {
                    format,
                    offset,
                    shader_location: location as u32,
                }
            })
            .collect();

        VertexBufferLayout {
            array_stride: stride,
            step_mode: mode,
            attributes,
        }
    }
}

impl From<MeshAttributeKind> for VertexAttribute {
    fn from(value: MeshAttributeKind) -> Self {
        match value {
            MeshAttributeKind::Position => VertexAttribute::Vec3,
            MeshAttributeKind::Normal => VertexAttribute::Vec3,
            MeshAttributeKind::TexCoord0 => VertexAttribute::Vec2,
            MeshAttributeKind::TexCoord1 => VertexAttribute::Vec2,
            MeshAttributeKind::Tangent => VertexAttribute::Vec3,
            MeshAttributeKind::Color => VertexAttribute::Color,
        }
    }
}
