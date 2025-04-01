use crate::{
    BindGroup, BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder, BlendMode, Buffer,
    DepthWrite, FragmentState, Id, Material, MaterialBinding, MaterialLayout, MaterialPhase, Mesh,
    MeshLayout, PipelineCache, PipelineId, RenderAssets, RenderDevice, RenderMesh,
    RenderPipelineDesc, RenderResource, RenderSurface, Shader, ShaderPath, SubMesh,
    UniformBufferArray, VertexState,
};
use asset::{AssetDatabase, AssetId, AssetRef};
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
use glam::Mat4;
use spatial::RangeFinder;
use std::{any::TypeId, collections::HashMap, ops::Range};
use wgpu::{BufferUsages, ColorTargetState, ShaderStages, VertexFormat, VertexStepMode};

use super::{PassBuilder, RenderContext, RenderGraphPass, RenderState};

pub trait ViewData: ShaderType + WriteInto + Sized + Send + Sync + 'static {
    type Query: BaseQuery;

    fn world(&self) -> &Mat4;

    fn extract<'a>(query: <Self::Query as BaseQuery>::Item<'a>) -> ExtractedView<Self>;
}

pub struct ExtractedView<V: ViewData> {
    pub entity: Entity,
    pub view: V,
    pub depth: i32,
}

pub struct ExtractedViews<V: ViewData>(pub(crate) Vec<ExtractedView<V>>);
impl<V: ViewData> Resource for ExtractedViews<V> {}
impl<V: ViewData> Default for ExtractedViews<V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct RenderView<V: ViewData> {
    entity: Entity,
    view: V,
    depth: i32,
    dynamic_offset: u32,
}

impl<V: ViewData> RenderView<V> {
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

    pub fn world(&self) -> &Mat4 {
        self.view.world()
    }

    pub fn dynamic_offset(&self) -> u32 {
        self.dynamic_offset
    }
}

pub struct ViewBuffer<V: ViewData> {
    views: Vec<RenderView<V>>,
    buffer: UniformBufferArray<V>,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
}

impl<V: ViewData> ViewBuffer<V> {
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

    pub(crate) fn extract(query: Main<Query<V::Query>>, mut views: ResMut<ExtractedViews<V>>) {
        for item in query.into_inner() {
            let view = V::extract(item);
            views.0.push(view);
        }

        views.0.sort_by(|a, b| a.depth.cmp(&b.depth));
    }

    pub(crate) fn queue(
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

impl<V: ViewData> Resource for ViewBuffer<V> {}
impl<V: ViewData> RenderResource for ViewBuffer<V> {
    type Arg = ReadRes<RenderDevice>;

    fn extract(
        device: ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self, crate::resources::ExtractError<()>> {
        Ok(Self::new(&device))
    }
}

pub trait MeshData: ShaderType + WriteInto + Send + Sync + 'static {}

impl<S: ShaderType + WriteInto + Send + Sync + 'static> MeshData for S {}

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

pub trait Renderer {
    type View: ViewData;
    type Mesh: MeshData;

    fn vertex_layout() -> &'static [VertexFormat];
    fn instance_layout() -> &'static [VertexFormat];

    fn primitive_state() -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        }
    }

    fn shader() -> impl Into<ShaderPath>;
}

pub type DrawMesh<D> = <<D as Draw>::Renderer as Renderer>::Mesh;
pub type DrawView<D> = <<D as Draw>::Renderer as Renderer>::View;

pub trait Draw: Send + Sync + 'static {
    type View: ViewData;
    type Material: Material;
    type Renderer: Renderer<View = Self::View>;
    type Query: BaseQuery;

    const BATCH: bool = true;

    fn entity(&self) -> Entity;

    fn data(&self) -> DrawMesh<Self>;

    fn material(&self) -> AssetRef<Self::Material>;

    fn mesh(&self) -> AssetRef<Mesh>;

    fn sub_mesh(&self) -> Option<SubMesh> {
        None
    }

    fn extract<'a>(query: <Self::Query as BaseQuery>::Item<'a>) -> Self;
}

pub struct Draws<D: Draw>(pub(crate) Vec<D>);
impl<D: Draw> Draws<D> {
    pub(crate) fn extract(query: Main<Query<D::Query>>, mut draws: ResMut<Self>) {
        for item in query.into_inner() {
            let draw = D::extract(item);
            draws.0.push(draw);
        }
    }

    pub(crate) fn clear_draws(mut draws: ResMut<Self>) {
        draws.0.clear();
    }
}

impl<D: Draw> Default for Draws<D> {
    fn default() -> Self {
        Self(Vec::new())
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

impl<D: Draw> AsRef<Vec<D>> for Draws<D> {
    fn as_ref(&self) -> &Vec<D> {
        &self.0
    }
}

impl<D: Draw> AsMut<Vec<D>> for Draws<D> {
    fn as_mut(&mut self) -> &mut Vec<D> {
        &mut self.0
    }
}

impl<D: Draw> Resource for Draws<D> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawKey {
    pub material: AssetId,
    pub mesh: AssetRef<Mesh>,
    pub sub_mesh: Option<SubMesh>,
}

pub struct DrawCall<M: MaterialPhase> {
    pub id: DrawId,
    pub key: DrawKey,
    pub item: M::Item,
    pub instances: Range<u32>,
}

pub trait IntoDrawCall<M: MaterialPhase> {
    fn into_draw_call(&self, range: &RangeFinder) -> M::Item;
}

pub struct ViewDrawCalls<V: ViewData, M: MaterialPhase>(
    pub(crate) HashMap<Entity, Vec<DrawCall<M>>>,
    std::marker::PhantomData<V>,
);

impl<V: ViewData, M: MaterialPhase> ViewDrawCalls<V, M> {
    pub(crate) fn queue<D: Draw<View = V> + IntoDrawCall<M>>(
        draws: Res<Draws<D>>,
        views: Res<ViewBuffer<D::View>>,
        draw_functions: Res<DrawFunctions<DrawView<D>>>,
        mut mesh_buffer: ResMut<MeshDataBuffer<DrawMesh<D>>>,
        mut view_draws: ResMut<ViewDrawCalls<V, M>>,
    ) {
        let draw_id = draw_functions
            .get_id::<D>()
            .expect("Draw function not found");

        for view in views.views() {
            let range_finder = RangeFinder::new(view.world().inverse().z_axis);

            if D::BATCH && <D::Material as Material>::Phase::mode() != BlendMode::Transparent {
                let mut batches = HashMap::new();

                for (index, call) in draws.0.iter().enumerate() {
                    let key = DrawKey {
                        material: call.material().into(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };

                    let (draw_index, data) = batches.entry(key).or_insert((index, vec![]));
                    data.push(call.data());

                    *draw_index = (*draw_index).min(index)
                }

                let draw_calls = batches.drain().map(|(key, (draw_index, data))| {
                    let instances = mesh_buffer.append(data);
                    let item = IntoDrawCall::<M>::into_draw_call(&draws[draw_index], &range_finder);

                    DrawCall {
                        id: draw_id,
                        key,
                        item,
                        instances,
                    }
                });

                view_draws
                    .0
                    .entry(view.entity())
                    .or_default()
                    .extend(draw_calls);
            } else {
                let draw_calls = draws.iter().map(|call| {
                    let offset = mesh_buffer.push(call.data());
                    let key = DrawKey {
                        material: call.material().into(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };

                    let item = IntoDrawCall::<M>::into_draw_call(call, &range_finder);
                    let instances = offset..(offset + 1);
                    DrawCall {
                        id: draw_id,
                        key,
                        item,
                        instances,
                    }
                });

                view_draws
                    .0
                    .entry(view.entity())
                    .or_default()
                    .extend(draw_calls);
            }
        }
    }

    pub(crate) fn clear_draws(mut view_draws: ResMut<Self>) {
        view_draws.0.clear();
    }

    pub fn draw(
        &self,
        state: &mut RenderState,
        ctx: &RenderContext,
        draw_functions: &DrawFunctions<V>,
        meshes: &RenderAssets<RenderMesh>,
        view_buffer: &ViewBuffer<V>,
        view: &RenderView<V>,
    ) {
        let Some(draws) = self.0.get(&view.entity()) else {
            return;
        };

        for draw in draws {
            let function = draw_functions.get(draw.id);
            function(
                state,
                ctx,
                meshes,
                view_buffer,
                view,
                &draw.key,
                draw.instances.clone(),
            );
        }
    }
}

impl<V: ViewData, M: MaterialPhase> Default for ViewDrawCalls<V, M> {
    fn default() -> Self {
        Self(HashMap::new(), std::marker::PhantomData)
    }
}

impl<V: ViewData, M: MaterialPhase> AsRef<HashMap<Entity, Vec<DrawCall<M>>>>
    for ViewDrawCalls<V, M>
{
    fn as_ref(&self) -> &HashMap<Entity, Vec<DrawCall<M>>> {
        &self.0
    }
}

impl<V: ViewData, M: MaterialPhase> Resource for ViewDrawCalls<V, M> {}

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
        Option<ReadRes<ViewBuffer<DrawView<D>>>>,
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

        let vertex_shader: Id<Shader> = match ShaderPath::new(D::Renderer::shader()) {
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
            D::Renderer::vertex_layout(),
            VertexStepMode::Vertex,
        )];

        if D::Renderer::instance_layout().len() > 0 {
            vertex_buffer_layouts.push(MeshLayout::into_vertex_buffer_layout(
                vertex_buffer_layouts[0].attributes.len() as u32,
                D::Renderer::instance_layout(),
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
                    blend: Some(<D::Material as Material>::Phase::mode().into()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: D::Renderer::primitive_state(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format(),
                depth_write_enabled: matches!(
                    <D::Material as Material>::Phase::depth_write(),
                    DepthWrite::On
                ),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawId(u32);

pub type DrawFunction<V> = fn(
    &mut RenderState,
    &RenderContext,
    &RenderAssets<RenderMesh>,
    &ViewBuffer<V>,
    &RenderView<V>,
    &DrawKey,
    Range<u32>,
);

pub struct DrawFunctions<V: ViewData>(IndexMap<TypeId, DrawFunction<V>>);
impl<V: ViewData> Default for DrawFunctions<V> {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

impl<V: ViewData> DrawFunctions<V> {
    pub fn add<D: Draw<View = V>>(&mut self) {
        let id = TypeId::of::<D>();
        if self.0.contains_key(&id) {
            return;
        }

        self.0.insert(
            id,
            |state, ctx, meshes, view_buffer, view, key, instances| {
                const VIEW_GROUP: u32 = 0;
                const MATERIAL_GROUP: u32 = 1;
                const VERTEX_BUFFER_SLOT: u32 = 0;
                const INSTANCE_BUFFER_SLOT: u32 = 1;

                let mesh_data = ctx.world().resource::<MeshDataBuffer<DrawMesh<D>>>();

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
            },
        );
    }

    pub fn get(&self, id: DrawId) -> DrawFunction<V> {
        self.0[id.0 as usize]
    }

    pub fn get_id<D: Draw<View = V>>(&self) -> Option<DrawId> {
        self.0
            .get_index_of(&TypeId::of::<D>())
            .map(|index| DrawId(index as u32))
    }
}

impl<V: ViewData> Resource for DrawFunctions<V> {}

pub trait ViewPass: Send + Sync + 'static {
    type View: ViewData;

    const NAME: super::Name;

    fn setup(
        builder: &mut PassBuilder,
    ) -> impl Fn(&mut RenderContext, &RenderView<Self::View>, &ViewBuffer<Self::View>) + 'static;
}

pub struct ViewPassNode<V: ViewPass>(std::marker::PhantomData<V>);
impl<V: ViewPass> ViewPassNode<V> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<V: ViewPass> RenderGraphPass for ViewPassNode<V> {
    const NAME: super::Name = V::NAME;

    fn setup(self, builder: &mut super::PassBuilder) -> impl Fn(&mut RenderContext) + 'static {
        let execute = V::setup(builder);

        move |ctx| {
            let view_buffer = ctx.world().resource::<ViewBuffer<V::View>>();

            for view in view_buffer.views() {
                execute(ctx, view, view_buffer);
            }
        }
    }
}
