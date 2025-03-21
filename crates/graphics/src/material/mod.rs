use crate::{
    extract::{RenderAsset, RenderAssets},
    renderer::{state::RenderState, RenderContext, RenderGraphNode, RenderPass},
    resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutBuilder, Buffer, CreateBindGroup, Id, Mesh,
        MeshAttributeKind, RenderMesh, RenderPipeline, RenderTarget, SubMesh, UniformBufferArray,
    },
    Color, RenderDevice, Viewport,
};
use asset::{asset::Asset, io::cache::LoadPath, AssetRef};
use bytemuck::{Pod, Zeroable};
use ecs::{Entity, Resource};
use encase::{internal::WriteInto, ShaderType};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, ops::Range};
use wgpu::{BindGroupEntry, BufferUsages, PrimitiveState, ShaderStages, VertexFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl BlendMode {
    pub fn blend_state(&self) -> wgpu::BlendState {
        match self {
            BlendMode::Opaque => wgpu::BlendState::REPLACE,
            BlendMode::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

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
    target: Option<Id<RenderTarget>>,
    dynamic_offset: u32,
}

impl<V: View> RenderView<V> {
    pub fn new(
        entity: Entity,
        view: V,
        depth: i32,
        viewport: Option<Viewport>,
        clear_color: Option<Color>,
        target: Option<Id<RenderTarget>>,
        dynamic_offset: u32,
    ) -> Self {
        Self {
            entity,
            view,
            depth,
            viewport,
            clear_color,
            target,
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

    pub fn target(&self) -> Option<Id<RenderTarget>> {
        self.target
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
        let alignment = device
            .limits()
            .min_uniform_buffer_offset_alignment
            .max(V::min_size().get() as u32);

        const INITIAL_BUFFER_SIZE: usize = 256 * 64;

        let mut buffer = UniformBufferArray::new_with_alignment(alignment as u64);
        buffer.reserve(INITIAL_BUFFER_SIZE / alignment as usize);

        let bind_group_layout = BindGroupLayoutBuilder::new()
            .with_uniform_buffer(0, ShaderStages::all(), true, None, None)
            .build(device);

        let bind_group = BindGroup::create(
            device,
            &bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.binding().unwrap(),
            }],
            (),
        );

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
        target: Option<Id<RenderTarget>>,
        viewport: Option<Viewport>,
    ) {
        let dynamic_offset = self.buffer.push(&view) as u32;
        self.views.push(RenderView::new(
            entity,
            view,
            depth,
            viewport,
            clear_color,
            target,
            dynamic_offset,
        ));
    }

    pub fn clear(&mut self) {
        self.views.clear();
        self.buffer.reset(64);
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

pub trait Material: Asset + CreateBindGroup + Send + Sync + 'static {
    fn mode() -> BlendMode;
    fn shader() -> impl Into<LoadPath>;
}

pub struct MaterialLayout<M: Material> {
    layout: BindGroupLayout,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> std::ops::Deref for MaterialLayout<M> {
    type Target = BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

pub struct MaterialBinding<M: Material> {
    bind_group: BindGroup,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> std::ops::Deref for MaterialBinding<M> {
    type Target = BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

impl<M: Material> RenderAsset for MaterialBinding<M> {
    type Id = AssetRef<M>;
}

pub trait Draw: Send + Sync + 'static {
    type Material: Material;
    type View: View;
    type Mesh: MeshData;
    type Pass: DrawPass<View = Self::View>;

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

    fn shader() -> impl Into<LoadPath>;
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

pub struct DrawPipeline<D: Draw>(RenderPipeline, std::marker::PhantomData<D>);

impl<D: Draw> std::ops::Deref for DrawPipeline<D> {
    type Target = RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Draw> Resource for DrawPipeline<D> {}

fn prepare_draw_calls<D: Draw>(
    draw_calls: &DrawCalls<D>,
    views: &ViewBuffer<D::View>,
    view_draw_calls: &mut ViewDrawCalls<D>,
    mesh_buffer: &mut MeshDataBuffer<D::Mesh>,
) {
    for view in views.views() {
        if D::BATCH && D::Material::mode() == BlendMode::Opaque {
            let mut batches = HashMap::new();

            for call in draw_calls.iter() {
                let key = BatchKey {
                    material: call.material(),
                    mesh: call.mesh(),
                    sub_mesh: call.sub_mesh(),
                };

                batches.entry(key).or_insert(vec![]).push(call.data());
            }

            view_draw_calls.ranges.insert(
                view.entity(),
                DrawRanges {
                    batches: batches
                        .drain()
                        .map(|(key, data)| {
                            let range = mesh_buffer.append(data);
                            (key, range)
                        })
                        .collect(),
                    unbatched: vec![],
                },
            );
        } else {
            let mut unbatched = vec![];

            for call in draw_calls.iter() {
                let offset = mesh_buffer.push(call.data());
                let key = BatchKey {
                    material: call.material(),
                    mesh: call.mesh(),
                    sub_mesh: call.sub_mesh(),
                };
                unbatched.push((key, offset));
            }

            view_draw_calls.ranges.insert(
                view.entity(),
                DrawRanges {
                    batches: HashMap::new(),
                    unbatched,
                },
            );
        }
    }
}

pub type DrawNode<V> = Box<
    dyn Fn(
            &RenderContext,
            &RenderView<V>,
            &ViewBuffer<V>,
            &RenderAssets<RenderMesh>,
            &mut RenderState,
        ) + Send
        + Sync,
>;

pub trait DrawPass: Send + Sync + 'static {
    type View: View;

    fn pass() -> RenderPass;
}

pub struct MaterialPassNode<P: DrawPass> {
    pass: RenderPass,
    opaque_pass: Vec<DrawNode<P::View>>,
    transparent_pass: Vec<DrawNode<P::View>>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: DrawPass> MaterialPassNode<P> {
    const VIEW_GROUP: u32 = 0;
    const MATERIAL_GROUP: u32 = 1;
    const VERTEX_BUFFER_SLOT: u32 = 0;
    const INSTANCE_BUFFER_SLOT: u32 = 1;

    pub fn new() -> Self {
        Self {
            pass: P::pass(),
            opaque_pass: Vec::new(),
            transparent_pass: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn add_node<D: Draw<View = P::View>>(&mut self) {
        let sub_pass = match D::Material::mode() {
            BlendMode::Opaque => &mut self.opaque_pass,
            BlendMode::Transparent => &mut self.transparent_pass,
        };

        let node = |ctx: &RenderContext,
                    view: &RenderView<D::View>,
                    view_buffer: &ViewBuffer<D::View>,
                    meshes: &RenderAssets<RenderMesh>,
                    state: &mut RenderState| {
            let draw_calls = ctx.resource::<ViewDrawCalls<D>>();
            let Some(draw_calls) = draw_calls.get(view.entity()) else {
                return;
            };

            let pipeline = ctx.resource::<DrawPipeline<D>>();
            let mesh_data = ctx.resource::<MeshDataBuffer<D::Mesh>>();
            let materials = ctx.resource::<RenderAssets<MaterialBinding<D::Material>>>();

            state.set_pipeline(pipeline);
            state.set_vertex_buffer(Self::INSTANCE_BUFFER_SLOT, mesh_data.buffer().slice(..));
            state.set_bind_group(
                Self::VIEW_GROUP,
                view_buffer.bind_group(),
                &[view.dynamic_offset()],
            );

            let mut execute = |key: &BatchKey<D::Material>, batch: Range<u32>| {
                let mesh = match meshes.get(&key.mesh) {
                    Some(mesh) => mesh,
                    None => return,
                };

                let material = match materials.get(&key.material) {
                    Some(material) => material,
                    None => return,
                };

                let (vertices, indices) = match key.sub_mesh {
                    Some(sub_mesh) => {
                        let vertices = sub_mesh.start_vertex as u32
                            ..(sub_mesh.start_vertex + sub_mesh.vertex_count) as u32;
                        let indices = sub_mesh.start_index as u32
                            ..(sub_mesh.start_index + sub_mesh.index_count) as u32;
                        (vertices, indices)
                    }
                    None => (0..mesh.vertex_count() as u32, 0..mesh.index_count() as u32),
                };

                state.set_vertex_buffer(Self::VERTEX_BUFFER_SLOT, mesh.vertex_buffer().slice(..));
                state.set_bind_group(Self::MATERIAL_GROUP, material, &[]);

                match mesh.index_buffer() {
                    Some(buffer) => {
                        state.set_index_buffer(buffer.slice(..));
                        state.draw_indexed(indices, vertices.start as i32, batch);
                    }
                    None => {
                        state.draw(vertices, batch);
                    }
                }
            };

            for (key, batch) in &draw_calls.batches {
                execute(key, batch.clone());
            }

            for (key, offset) in &draw_calls.unbatched {
                let batch = *offset..offset + 1;
                execute(key, batch);
            }
        };

        sub_pass.push(Box::new(node));
    }
}

impl<P: DrawPass> RenderGraphNode for MaterialPassNode<P> {
    fn name(&self) -> &str {
        "MaterialPass"
    }

    fn run(&mut self, ctx: &mut crate::renderer::RenderContext) {
        let mut encoder = ctx.encoder();

        let views = ctx.resource::<ViewBuffer<P::View>>();
        let meshes = ctx.resource::<RenderAssets<RenderMesh>>();

        for view in views.views() {
            let target = view.target().and_then(|id| ctx.get_render_target(id));
            let clear = view.clear_color();

            if let Some(mut pass) = self.pass.begin(&mut encoder, ctx, target, clear) {
                let mut state = RenderState::new(&mut pass);

                for node in &self.opaque_pass {
                    node(ctx, view, views, meshes, &mut state);
                }

                for node in &self.transparent_pass {
                    node(ctx, view, views, meshes, &mut state);
                }
            }
        }
    }
}
