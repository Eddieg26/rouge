use super::{PassBuilder, RenderGraphPass};
use crate::{
    device::RenderDevice,
    renderer::{graph::RenderContext, state::RenderState},
    resources::{
        BlendMode, FragmentState, Id, Material, MaterialBinding, MaterialLayout, Mesh, MeshLayout,
        PipelineCache, PipelineId, RenderAssets, RenderMesh, RenderPipelineDesc, RenderResource,
        Shader, ShaderPath, SubMesh, VertexState,
        binding::{BindGroup, BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder},
        buffer::{Buffer, UniformBufferArray},
    },
    surface::RenderSurface,
};
use asset::{AssetId, AssetRef, database::AssetDatabase};
use ecs::{
    Entity, IndexMap, Res, ResMut, Resource,
    system::unlifetime::{ReadRes, WriteRes},
    world::{
        action::WorldActions,
        builtin::actions::AddResource,
        query::{BaseQuery, Query},
    },
};
use encase::{ShaderType, internal::WriteInto};
use game::Main;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::{any::TypeId, collections::HashMap, hash::Hash, ops::Range};
use wgpu::{
    BufferUsages, ColorTargetState, PrimitiveState, ShaderStages, VertexFormat, VertexStepMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepthWrite {
    On,
    Off,
}

#[derive(Debug, Clone, Default)]
pub struct ViewEntities(pub(crate) Vec<Entity>);
impl Resource for ViewEntities {}

pub trait View: ShaderType + WriteInto + Send + Sync + Sized + 'static {
    type Query: BaseQuery;

    fn extract<'a>(query: <Self::Query as BaseQuery>::Item<'a>) -> ExtractedView<Self>;
}

pub struct ExtractedView<V: View> {
    pub entity: Entity,
    pub depth: i32,
    pub view: V,
}

pub struct ExtractedViews<V: View>(pub(crate) Vec<ExtractedView<V>>);
impl<V: View> Resource for ExtractedViews<V> {}
impl<V: View> Default for ExtractedViews<V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct RenderView<V: View> {
    entity: Entity,
    view: V,
    depth: i32,
    dynamic_offset: u32,
}

impl<V: View> RenderView<V> {
    pub fn new(entity: Entity, view: V, depth: i32, dynamic_offset: u32) -> Self {
        Self {
            entity,
            view,
            depth,
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
            .with_uniform(0, ShaderStages::all(), true, None, None)
            .build(device);

        let bind_group = BindGroupBuilder::new(&bind_group_layout)
            .with_uniform(0, buffer.as_ref(), 0, None)
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

    pub fn get_view(&self, entity: Entity) -> Option<&RenderView<V>> {
        self.views.iter().find(|view| view.entity() == entity)
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

    pub fn add_view(&mut self, extracted: ExtractedView<V>) {
        let dynamic_offset = self.buffer.push(&extracted.view) as u32;
        self.views.push(RenderView::new(
            extracted.entity,
            extracted.view,
            extracted.depth,
            dynamic_offset,
        ));
    }

    pub fn clear(&mut self) {
        self.views.clear();
        self.buffer.clear();
    }

    pub fn sort(&mut self) {
        self.views.sort_by(|a, b| a.depth().cmp(&b.depth()));
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.buffer.update(device).is_some() {
            self.bind_group = BindGroupBuilder::new(&self.bind_group_layout)
                .with_uniform(0, self.buffer.as_ref(), 0, None)
                .build(device);
        }
    }

    pub(crate) fn extract_views(
        query: Main<Query<V::Query>>,
        mut views: ResMut<ExtractedViews<V>>,
        mut view_entites: ResMut<ViewEntities>,
    ) {
        for item in query.into_inner() {
            let view = V::extract(item);
            views.0.push(view);
        }

        view_entites
            .0
            .extend(views.0.iter().map(|view| view.entity));
    }

    pub(crate) fn queue_views(
        mut view_buffer: ResMut<ViewBuffer<V>>,
        mut extracted_views: ResMut<ExtractedViews<V>>,
    ) {
        for view in extracted_views.0.drain(..) {
            view_buffer.add_view(view);
        }

        view_buffer.sort();
    }

    pub(crate) fn clear_views(mut view_buffer: ResMut<ViewBuffer<V>>) {
        view_buffer.clear();
    }

    pub(crate) fn update_views(device: Res<RenderDevice>, mut view_buffer: ResMut<ViewBuffer<V>>) {
        view_buffer.update(&device);
    }
}

impl<V: View> Resource for ViewBuffer<V> {}
impl<V: View> RenderResource for ViewBuffer<V> {
    type Arg = ReadRes<RenderDevice>;

    fn extract(
        device: ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self, crate::resources::ExtractError<()>> {
        Ok(Self::new(&device))
    }
}

pub trait MeshData: ShaderType + WriteInto + Send + Sync + Sized + 'static {
    fn world(&self) -> Mat4;
    fn position(&self) -> Vec3 {
        let world = self.world();
        Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }
}

pub struct MeshDataBuffer<T: MeshData> {
    buffer: Buffer,
    data: Vec<u8>,
    offset: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: MeshData> MeshDataBuffer<T> {
    const SIZE: usize = std::mem::size_of::<T>();

    pub fn new(device: &RenderDevice) -> Self {
        let data = vec![0u8; std::mem::size_of::<T>()];
        let buffer = Buffer::with_data(
            device,
            &data,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            None,
        );

        Self {
            buffer,
            data,
            offset: 0,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn push(&mut self, data: T) -> u32 {
        let offset = self.offset;
        let mut writer = encase::internal::Writer::new(&data, &mut self.data, offset).unwrap();
        data.write_into(&mut writer);

        self.offset += Self::SIZE;

        (offset / Self::SIZE) as u32
    }

    pub fn append(&mut self, mut data: Vec<T>) -> Range<u32> {
        let offset = self.offset;
        data.drain(..).for_each(|data| {
            self.push(data);
        });
        (offset / Self::SIZE) as u32..(self.offset / Self::SIZE) as u32
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.offset = 0;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        let size = self.data.len() as u64;
        if size > self.buffer.size() {
            self.buffer.update(device, &self.data);
        } else if size > 0 && size < self.buffer.size() / 2 {
            self.buffer.resize_with_data(device, &self.data);
        } else if size > 0 {
            self.buffer.update(device, &self.data);
        } else {
            self.buffer.resize(device, Self::SIZE as u64);
        }
    }

    pub(crate) fn update_mesh_buffer(device: Res<RenderDevice>, mut buffer: ResMut<Self>) {
        buffer.update(&device);
    }

    pub(crate) fn clear_mesh_buffer(mut buffer: ResMut<Self>) {
        buffer.clear();
    }
}

impl<T: MeshData> Resource for MeshDataBuffer<T> {}
impl<T: MeshData> RenderResource for MeshDataBuffer<T> {
    type Arg = ReadRes<RenderDevice>;

    fn extract(
        arg: ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self, crate::resources::ExtractError<()>> {
        Ok(Self::new(&arg))
    }
}

pub struct ViewDrawCalls<D: DrawPhase>(HashMap<Entity, Vec<DrawCall<D>>>);
impl<D: DrawPhase> Default for ViewDrawCalls<D> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<D: DrawPhase> ViewDrawCalls<D> {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn get(&self, entity: Entity) -> Option<&Vec<DrawCall<D>>> {
        self.0.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut Vec<DrawCall<D>>> {
        self.0.get_mut(&entity)
    }

    pub fn insert(&mut self, entity: Entity, ranges: Vec<DrawCall<D>>) {
        self.0.insert(entity, ranges);
    }

    pub fn remove(&mut self, entity: Entity) -> Option<Vec<DrawCall<D>>> {
        self.0.remove(&entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Entity, &Vec<DrawCall<D>>)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn draw(
        &self,
        ctx: &RenderContext,
        view: &RenderView<D::View>,
        meshes: &RenderAssets<RenderMesh>,
        view_buffer: &ViewBuffer<D::View>,
        draw_functions: &DrawFunctions<D::View>,
        state: &mut RenderState,
    ) {
        if let Some(calls) = self.get(view.entity()) {
            for call in calls.iter() {
                draw_functions.get(call.id)(
                    &call.key,
                    call.instances.clone(),
                    ctx,
                    view,
                    view_buffer,
                    meshes,
                    state,
                );
            }
        }
    }
}

impl<D: DrawPhase> Resource for ViewDrawCalls<D> {}

pub struct Draws<D: Draw>(Vec<D>);
impl<D: Draw> Default for Draws<D> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<D: Draw> Draws<D> {
    pub(crate) fn extract_draws(query: Main<Query<D::Query>>, mut draws: ResMut<Self>) {
        for item in query.into_inner() {
            let draw = D::extract(item);
            draws.0.push(draw);
        }
    }

    pub(crate) fn queue_view_draws(
        draws: Res<Self>,
        views: Res<ViewBuffer<D::View>>,
        draw_functions: Res<DrawFunctions<D::View>>,
        mut view_draws: ResMut<ViewDrawCalls<D::Phase>>,
        mut mesh_buffer: ResMut<MeshDataBuffer<D::Mesh>>,
    ) {
        let draw_id = draw_functions.get_id::<D>();

        for view in views.views() {
            if D::BATCH && D::Material::mode() != BlendMode::Transparent {
                let mut batches = HashMap::new();

                for (index, call) in draws.iter().enumerate() {
                    let key = BatchKey {
                        material: call.material().into(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };

                    let (draw_index, data) = batches.entry(key).or_insert((index, vec![]));
                    data.push(call.data());

                    *draw_index = (*draw_index).min(index)
                }

                let calls = batches
                    .drain()
                    .map(|(key, (draw_index, data))| {
                        let instances = mesh_buffer.append(data);
                        let phase = draws[draw_index].phase(view);
                        DrawCall::new(draw_id, key, phase, instances)
                    })
                    .collect::<Vec<_>>();

                view_draws.insert(view.entity(), calls);
            } else {
                let mut unbatched = vec![];

                for call in draws.iter() {
                    let offset = mesh_buffer.push(call.data());
                    let key = BatchKey {
                        material: call.material().into(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };

                    unbatched.push(DrawCall {
                        id: draw_id,
                        key,
                        phase: call.phase(view),
                        instances: offset..offset + 1,
                    });
                }
            }
        }
    }

    pub(crate) fn clear_draws(mut draws: ResMut<Self>) {
        draws.0.clear();
    }
}

impl<D: Draw> std::ops::Deref for Draws<D> {
    type Target = Vec<D>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Draw> std::ops::DerefMut for Draws<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D: Draw> Resource for Draws<D> {}

pub struct DrawPipline<D: Draw>(PipelineId, std::marker::PhantomData<D>);
impl<D: Draw> DrawPipline<D> {
    pub(crate) fn new(id: PipelineId) -> Self {
        Self(id, std::marker::PhantomData)
    }
}

impl<D: Draw> std::ops::Deref for DrawPipline<D> {
    type Target = PipelineId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Draw> AsRef<PipelineId> for DrawPipline<D> {
    fn as_ref(&self) -> &PipelineId {
        &self.0
    }
}

impl<D: Draw> Resource for DrawPipline<D> {}
impl<D: Draw> RenderResource for DrawPipline<D> {
    type Arg = (
        WriteRes<PipelineCache>,
        ReadRes<AssetDatabase>,
        ReadRes<RenderDevice>,
        ReadRes<RenderSurface>,
        Option<ReadRes<ViewBuffer<D::View>>>,
        Option<ReadRes<MaterialLayout<D::Material>>>,
        WorldActions,
    );

    fn extract(
        arg: ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self, crate::resources::ExtractError<()>> {
        let (mut cache, database, device, surface, view, layout, actions) = arg;

        let Some(view) = view else {
            return Err(crate::resources::ExtractError::Retry(()));
        };

        let layout = match layout {
            Some(layout) => layout.value().clone(),
            None => {
                let layout = MaterialLayout::<D::Material>::new(&device);
                actions.add(AddResource::new(layout.clone()));

                layout
            }
        };

        let vertex_shader: Id<Shader> = match ShaderPath::new(D::shader()) {
            ShaderPath::Id(id) => (*id).into(),
            ShaderPath::Path(path) => database
                .path_id(&path.into())
                .ok_or(())
                .map_err(|_| crate::resources::ExtractError::Retry(()))?
                .into(),
        };

        let fragment_shader: Id<Shader> = match ShaderPath::new(D::Material::shader()) {
            ShaderPath::Id(id) => (*id).into(),
            ShaderPath::Path(path) => database
                .path_id(&path.into())
                .ok_or(())
                .map_err(|_| crate::resources::ExtractError::Retry(()))?
                .into(),
        };

        let mut vertex_buffer_layouts = vec![MeshLayout::into_vertex_buffer_layout(
            0,
            D::vertex_layout(),
            VertexStepMode::Vertex,
        )];

        if D::instance_layout().len() > 0 {
            vertex_buffer_layouts.push(MeshLayout::into_vertex_buffer_layout(
                vertex_buffer_layouts[0].attributes.len() as u32,
                D::instance_layout(),
                VertexStepMode::Instance,
            ));
        }

        let id = cache.queue_render_pipeline(RenderPipelineDesc {
            label: None,
            layout: vec![view.bind_group_layout().clone(), layout.as_ref().clone()],
            vertex: VertexState {
                shader: vertex_shader,
                entry: "main".into(),
                buffers: vertex_buffer_layouts,
            },
            fragment: Some(FragmentState {
                shader: fragment_shader,
                entry: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: surface.format(),
                    blend: Some(D::Material::mode().into()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: D::primitive(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format(),
                depth_write_enabled: matches!(D::depth_write(), DepthWrite::On),
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            push_constants: vec![],
        });

        Ok(Self::new(id))
    }
}

pub type DrawFunction<V> = fn(
    &BatchKey,
    Range<u32>,
    &RenderContext,
    &RenderView<V>,
    &ViewBuffer<V>,
    &RenderAssets<RenderMesh>,
    &mut RenderState,
);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct DrawId(u32);

pub struct DrawFunctions<V: View>(IndexMap<TypeId, DrawFunction<V>>);
impl<V: View> Default for DrawFunctions<V> {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

impl<V: View> DrawFunctions<V> {
    pub fn add<D: Draw<View = V>>(&mut self) -> DrawId {
        let id = TypeId::of::<D>();
        if !self.0.contains_key(&id) {
            self.0.insert(id, Self::draw_function::<D>());
        }

        self.0
            .get_index_of(&id)
            .map(|index| DrawId(index as u32))
            .unwrap()
    }

    pub fn get(&self, id: DrawId) -> &DrawFunction<V> {
        &self.0[id.0 as usize]
    }

    pub fn get_id<D: Draw<View = V>>(&self) -> DrawId {
        let id = TypeId::of::<D>();
        self.0
            .get_index_of(&id)
            .map(|index| DrawId(index as u32))
            .unwrap()
    }

    fn draw_function<D: Draw<View = V>>() -> DrawFunction<V> {
        const VIEW_GROUP: u32 = 0;
        const MATERIAL_GROUP: u32 = 1;
        const VERTEX_BUFFER_SLOT: u32 = 0;
        const INSTANCE_BUFFER_SLOT: u32 = 1;

        |key,
         instances,
         ctx,
         view,
         view_buffer: &ViewBuffer<D::View>,
         meshes: &RenderAssets<RenderMesh>,
         state: &mut RenderState| {
            let mesh_data = ctx.world().resource::<MeshDataBuffer<D::Mesh>>();

            let mesh = match meshes.get(&(*key.mesh).into()) {
                Some(mesh) => mesh,
                None => return,
            };

            let materials = ctx
                .world()
                .resource::<RenderAssets<MaterialBinding<D::Material>>>();

            let material = match materials.get(&(key.material.into())) {
                Some(material) => material,
                None => return,
            };

            let Some(pipeline) = ctx
                .world()
                .try_resource::<DrawPipline<D>>()
                .and_then(|id| ctx.get_render_pipeline(id))
            else {
                return;
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

            state.set_pipeline(pipeline);
            state.set_vertex_buffer(INSTANCE_BUFFER_SLOT, mesh_data.buffer().slice(..));
            state.set_bind_group(
                VIEW_GROUP,
                view_buffer.bind_group(),
                &[view.dynamic_offset()],
            );

            state.set_vertex_buffer(VERTEX_BUFFER_SLOT, mesh.vertex_buffer().slice(..));
            state.set_bind_group(MATERIAL_GROUP, material, &[]);

            match mesh.index_buffer() {
                Some(buffer) => {
                    state.set_index_buffer(buffer.slice(..));
                    state.draw_indexed(indices, vertices.start as i32, instances);
                }
                None => {
                    state.draw(vertices, instances);
                }
            }
        }
    }
}

impl<V: View> Resource for DrawFunctions<V> {}

pub trait DrawPhase: Send + Sync + 'static {
    type View: View;
}

pub trait SortedPhase: 'static {
    type Key: Hash + Eq + PartialOrd + Ord;

    fn key(&self) -> Self::Key;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BatchKey {
    pub material: AssetId,
    pub mesh: AssetRef<Mesh>,
    pub sub_mesh: Option<SubMesh>,
}

pub struct DrawCall<D: DrawPhase> {
    pub id: DrawId,
    pub key: BatchKey,
    pub phase: D,
    pub instances: Range<u32>,
}

impl<D: DrawPhase> DrawCall<D> {
    pub fn new(id: DrawId, key: BatchKey, phase: D, instances: Range<u32>) -> Self {
        Self {
            id,
            key,
            phase,
            instances,
        }
    }
}

pub trait ViewPass: Send + Sync + 'static {
    type View: View;

    const NAME: super::Name;

    fn setup(
        builder: &mut PassBuilder,
    ) -> impl Fn(&mut RenderContext, &RenderView<Self::View>) + 'static;
}

pub struct ViewPassNode<V: ViewPass>(std::marker::PhantomData<V>);

impl<V: ViewPass> RenderGraphPass for ViewPassNode<V> {
    const NAME: super::Name = V::NAME;

    fn setup(self, builder: &mut super::PassBuilder) -> impl Fn(&mut RenderContext) + 'static {
        let execute = V::setup(builder);

        move |ctx| {
            let view_buffer = ctx.world().resource::<ViewBuffer<V::View>>();

            for view in view_buffer.views() {
                execute(ctx, view);
            }
        }
    }
}

pub trait Draw: Send + Sync + 'static {
    type View: View;
    type Mesh: MeshData;
    type Material: Material;
    type Phase: DrawPhase;
    type Query: BaseQuery;

    const BATCH: bool = true;
    const CULL: bool = true;

    fn entity(&self) -> Entity;

    fn data(&self) -> Self::Mesh;

    fn material(&self) -> AssetRef<Self::Material>;

    fn mesh(&self) -> AssetRef<Mesh>;

    fn sub_mesh(&self) -> Option<SubMesh> {
        None
    }

    fn phase(&self, view: &RenderView<Self::View>) -> Self::Phase;

    fn shader() -> impl Into<ShaderPath>;

    fn vertex_layout() -> &'static [VertexFormat];

    fn instance_layout() -> &'static [VertexFormat] {
        &[]
    }

    fn primitive() -> PrimitiveState {
        PrimitiveState::default()
    }

    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }

    fn extract<'a>(query: <Self::Query as BaseQuery>::Item<'a>) -> Self;
}
