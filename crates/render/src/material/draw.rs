use crate::{
    device::RenderDevice,
    renderer::{graph::RenderContext, pass::RenderPass, state::RenderState},
    resources::{
        Mesh, MeshAttributeKind, RenderAssets, RenderMesh, ShaderPath, SubMesh,
        binding::{BindGroup, BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder},
        buffer::{Buffer, UniformBufferArray},
    },
    types::{Color, Viewport},
};
use asset::AssetRef;
use bytemuck::{Pod, Zeroable};
use ecs::{Entity, Resource};
use encase::{ShaderType, internal::WriteInto};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, ops::Range};
use wgpu::{BufferUsages, PrimitiveState, ShaderStages, VertexFormat};

use super::Material;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepthWrite {
    On,
    Off,
    Auto,
}

pub trait View: ShaderType + WriteInto + Default + Send + Sync + 'static {
    fn world(&self) -> Mat4;
    fn view(&self) -> Mat4;
    fn projection(&self) -> Mat4;
    fn position(&self) -> Vec3 {
        let world = self.world();
        Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }
}

pub struct RenderView<V: View> {
    entity: Entity,
    view: V,
    depth: i32,
    viewport: Option<Viewport>,
    clear_color: Option<Color>,
    dynamic_offset: u32,
}

impl<V: View> RenderView<V> {
    pub fn new(
        entity: Entity,
        view: V,
        depth: i32,
        viewport: Option<Viewport>,
        clear_color: Option<Color>,
        dynamic_offset: u32,
    ) -> Self {
        Self {
            entity,
            view,
            depth,
            viewport,
            clear_color,
            dynamic_offset,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn view(&self) -> &V {
        &self.view
    }

    pub fn depth(&self) -> i32 {
        self.depth
    }

    pub fn viewport(&self) -> Option<&Viewport> {
        self.viewport.as_ref()
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    pub fn dynamic_offset(&self) -> u32 {
        self.dynamic_offset
    }
}

pub struct ViewBuffer<V: View> {
    views: Vec<RenderView<V>>,
    buffer: UniformBufferArray<V>,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
}

impl<V: View> ViewBuffer<V> {
    pub fn new(device: &RenderDevice) -> Self {
        let buffer = UniformBufferArray::new(device, None, Some(BufferUsages::COPY_DST));

        let bind_group_layout = BindGroupLayoutBuilder::new()
            .uniform(ShaderStages::all(), true, None, None)
            .build(device);

        let bind_group = BindGroupBuilder::new(&bind_group_layout)
            .uniform(0, buffer.as_ref(), 0, None)
            .build(device);

        Self {
            views: Vec::new(),
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn views(&self) -> &[RenderView<V>] {
        &self.views
    }

    pub fn buffer(&self) -> &UniformBufferArray<V> {
        &self.buffer
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn add_view(
        &mut self,
        entity: Entity,
        view: V,
        depth: i32,
        clear_color: Option<Color>,
        viewport: Option<Viewport>,
    ) {
        let dynamic_offset = self.buffer.push(&view) as u32;
        self.views.push(RenderView::new(
            entity,
            view,
            depth,
            viewport,
            clear_color,
            dynamic_offset,
        ));
    }

    pub fn clear(&mut self) {
        self.views.clear();
        self.buffer.clear();
    }

    pub fn update(&mut self, device: &RenderDevice) {
        self.buffer.update(device);
    }
}

impl<V: View> Resource for ViewBuffer<V> {}

pub trait MeshData: Pod + Zeroable + Default + Send + Sync + 'static {
    fn world(&self) -> Mat4;
    fn position(&self) -> Vec3 {
        let world = self.world();
        Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }
}

pub struct MeshDataBuffer<T: MeshData> {
    buffer: Buffer,
    instances: Vec<T>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: MeshData> MeshDataBuffer<T> {
    pub fn new(device: &RenderDevice) -> Self {
        let buffer = Buffer::with_data(
            device,
            &vec![0u8; std::mem::size_of::<T>()],
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            None,
        );

        Self {
            buffer,
            instances: vec![],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn push(&mut self, data: T) -> u32 {
        let len = self.instances.len() as u32;
        self.instances.push(data);
        len
    }

    pub fn append(&mut self, data: Vec<T>) -> Range<u32> {
        let start = self.instances.len() as u32;
        self.instances.extend(data);
        start..self.instances.len() as u32
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn update(&mut self, device: &RenderDevice) {
        let size = (self.instances.len() * std::mem::size_of::<T>()) as u64;
        if size > 0 && size > self.buffer.size() {
            self.buffer
                .update(device, bytemuck::cast_slice(&self.instances));
        } else if size > 0 && size < self.buffer.size() / 2 {
            self.buffer
                .resize_with_data(device, bytemuck::cast_slice(&self.instances));
        } else if size > 0 {
            self.buffer
                .update(device, bytemuck::cast_slice(&self.instances));
        }
    }
}

impl<T: MeshData> Resource for MeshDataBuffer<T> {}

#[derive(Clone, Copy)]
pub struct BatchKey<M: Material> {
    pub material: AssetRef<M>,
    pub mesh: AssetRef<Mesh>,
    pub sub_mesh: Option<SubMesh>,
}

impl<M: Material> PartialEq for BatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material
            && self.mesh == other.mesh
            && self.sub_mesh == other.sub_mesh
    }
}
impl<M: Material> Eq for BatchKey<M> {}
impl<M: Material> Hash for BatchKey<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.material.hash(state);
        self.mesh.hash(state);
        self.sub_mesh.hash(state);
    }
}

pub struct DrawRanges<D: Draw> {
    batches: HashMap<BatchKey<D::Material>, Range<u32>>,
    unbatched: Vec<(BatchKey<D::Material>, u32)>,
}

pub struct ViewDrawCalls<D: Draw> {
    ranges: HashMap<Entity, DrawRanges<D>>,
}

impl<D: Draw> ViewDrawCalls<D> {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    pub fn get(&self, entity: Entity) -> Option<&DrawRanges<D>> {
        self.ranges.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut DrawRanges<D>> {
        self.ranges.get_mut(&entity)
    }

    pub fn insert(&mut self, entity: Entity, ranges: DrawRanges<D>) {
        self.ranges.insert(entity, ranges);
    }

    pub fn remove(&mut self, entity: Entity) -> Option<DrawRanges<D>> {
        self.ranges.remove(&entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Entity, &DrawRanges<D>)> {
        self.ranges.iter()
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

impl<D: Draw> Resource for ViewDrawCalls<D> {}

pub struct DrawCalls<D: Draw>(Vec<D>);
impl<D: Draw> std::ops::Deref for DrawCalls<D> {
    type Target = Vec<D>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Draw> std::ops::DerefMut for DrawCalls<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D: Draw> Resource for DrawCalls<D> {}

pub type DrawPass<V> = Box<
    dyn Fn(
            &RenderContext,
            &RenderView<V>,
            &ViewBuffer<V>,
            &RenderAssets<RenderMesh>,
            &mut RenderState,
        ) + Send
        + Sync,
>;

pub trait MaterialPass: Send + Sync + 'static {
    type View: View;

    fn pass() -> RenderPass;
}

pub trait Draw: Send + Sync + 'static {
    type View: View;
    type Mesh: MeshData;
    type Material: Material;
    type Pass: MaterialPass<View = Self::View>;

    const BATCH: bool = true;
    const CULL: bool = true;
    const SORT: bool = false;

    fn entity(&self) -> Entity;

    fn data(&self) -> Self::Mesh;

    fn material(&self) -> AssetRef<Self::Material>;

    fn mesh(&self) -> AssetRef<Mesh>;

    fn sub_mesh(&self) -> Option<SubMesh> {
        None
    }

    fn shader() -> impl Into<ShaderPath>;

    fn mesh_attributes() -> &'static [MeshAttributeKind];

    fn instance_attributes() -> &'static [VertexFormat] {
        &[]
    }

    fn primitive() -> PrimitiveState {
        PrimitiveState::default()
    }

    fn depth_write() -> DepthWrite {
        DepthWrite::Auto
    }
}
