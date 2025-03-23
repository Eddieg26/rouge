use super::{GraphPass, TextureDesc, TextureView};
use crate::{
    device::RenderDevice,
    renderer::{graph::RenderContext, pass::RenderPass, state::RenderState},
    resources::{
        BlendMode, FragmentState, Id, Material, MaterialBinding, MaterialLayout, Mesh, MeshLayout,
        PipelineCache, PipelineId, RenderAssets, RenderMesh, RenderPipelineDesc, RenderResource,
        Shader, ShaderPath, SubMesh, VertexState,
        binding::{BindGroup, BindGroupBuilder, BindGroupLayout, BindGroupLayoutBuilder},
        buffer::{Buffer, UniformBufferArray},
    },
    surface::RenderSurface,
    types::{Color, Viewport},
};
use asset::{AssetRef, database::AssetDatabase};
use ecs::{
    Component, Entity, IndexMap, Res, ResMut, Resource,
    system::unlifetime::{Read, ReadRes, WriteRes},
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
use std::{collections::HashMap, hash::Hash, ops::Range};
use wgpu::{
    BufferUsages, ColorTargetState, PrimitiveState, ShaderStages, TextureUsages, VertexFormat,
    VertexStepMode,
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

    fn world(&self) -> Mat4;
    fn view(&self) -> Mat4;
    fn projection(&self) -> Mat4;
    fn position(&self) -> Vec3 {
        let world = self.world();
        Vec3::new(world.w_axis.x, world.w_axis.y, world.w_axis.z)
    }

    fn extract<'a>(query: <Self::Query as BaseQuery>::Item<'a>) -> ExtractedView<Self>;
}

#[derive(ShaderType, Clone, Copy)]
pub struct SimpleView {
    pub world: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
}

impl Default for SimpleView {
    fn default() -> Self {
        Self {
            world: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        }
    }
}

impl View for SimpleView {
    type Query = (Entity, Read<SimpleView>);

    fn world(&self) -> Mat4 {
        self.world
    }

    fn view(&self) -> Mat4 {
        self.view
    }

    fn projection(&self) -> Mat4 {
        self.projection
    }

    fn extract<'a>(item: <Self::Query as BaseQuery>::Item<'a>) -> ExtractedView<SimpleView> {
        ExtractedView {
            entity: item.0,
            view: *item.1,
            depth: 0,
            viewport: None,
            clear_color: None,
        }
    }
}

impl Component for SimpleView {}

pub struct ExtractedView<V: View> {
    pub entity: Entity,
    pub view: V,
    pub depth: i32,
    pub viewport: Option<Viewport>,
    pub clear_color: Option<Color>,
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
            let extracted = V::extract(item);
            views.0.push(extracted);
        }

        views.0.sort_by(|a, b| a.depth.cmp(&b.depth));
        view_entites
            .0
            .extend(views.0.iter().map(|view| view.entity));
    }

    pub(crate) fn queue_views(
        mut view_buffer: ResMut<ViewBuffer<V>>,
        mut extracted_views: ResMut<ExtractedViews<V>>,
    ) {
        for view in extracted_views.0.drain(..) {
            view_buffer.add_view(
                view.entity,
                view.view,
                view.depth,
                view.clear_color,
                view.viewport,
            );
        }
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

    pub fn push(&mut self, data: T) -> u32 {
        let offset = self.offset;
        let mut writer = encase::internal::Writer::new(&data, &mut self.data, offset).unwrap();
        data.write_into(&mut writer);

        self.offset += Self::SIZE;

        (offset / Self::SIZE) as u32
    }

    pub fn append(&mut self, mut data: Vec<T>) -> Range<u32> {
        let offset = self.offset;
        let _ = data.drain(..).map(|data| self.push(data));
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
    pub(crate) batches: HashMap<BatchKey<D::Material>, Range<u32>>,
    pub(crate) unbatched: Vec<(BatchKey<D::Material>, u32)>,
}

pub struct ViewDrawCalls<D: Draw>(HashMap<Entity, DrawRanges<D>>);
impl<D: Draw> Default for ViewDrawCalls<D> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<D: Draw> ViewDrawCalls<D> {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn get(&self, entity: Entity) -> Option<&DrawRanges<D>> {
        self.0.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut DrawRanges<D>> {
        self.0.get_mut(&entity)
    }

    pub fn insert(&mut self, entity: Entity, ranges: DrawRanges<D>) {
        self.0.insert(entity, ranges);
    }

    pub fn remove(&mut self, entity: Entity) -> Option<DrawRanges<D>> {
        self.0.remove(&entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Entity, &DrawRanges<D>)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<D: Draw> Resource for ViewDrawCalls<D> {}

pub struct DrawCalls<D: Draw>(Vec<D>);
impl<D: Draw> Default for DrawCalls<D> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<D: Draw> DrawCalls<D> {
    pub(crate) fn extract_draws(query: Main<Query<D::Query>>, mut draws: ResMut<Self>) {
        // draws.0.clear();
        for item in query.into_inner() {
            let draw = D::extract(item);
            draws.0.push(draw);
        }
    }

    pub(crate) fn queue_view_draws(
        draws: Res<Self>,
        views: Res<ViewBuffer<D::View>>,
        mut view_draws: ResMut<ViewDrawCalls<D>>,
        mut mesh_buffer: ResMut<MeshDataBuffer<D::Mesh>>,
    ) {
        for view in views.views() {
            if D::BATCH && D::Material::mode() != BlendMode::Transparent {
                let mut batches = HashMap::new();

                for call in draws.iter() {
                    let key = BatchKey {
                        material: call.material(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };

                    batches.entry(key).or_insert(vec![]).push(call.data());
                }

                view_draws.insert(
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

                for call in draws.iter() {
                    let offset = mesh_buffer.push(call.data());
                    let key = BatchKey {
                        material: call.material(),
                        mesh: call.mesh(),
                        sub_mesh: call.sub_mesh(),
                    };
                    unbatched.push((key, offset));
                }

                view_draws.insert(
                    view.entity(),
                    DrawRanges {
                        batches: HashMap::new(),
                        unbatched,
                    },
                );
            }
        }
    }

    pub(crate) fn clear_draws(mut draws: ResMut<Self>) {
        draws.0.clear();
    }
}

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

pub type DrawFunction<V> =
    fn(&RenderContext, &RenderView<V>, &ViewBuffer<V>, &RenderAssets<RenderMesh>, &mut RenderState);

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

pub struct DrawFunctions<M: DrawPass> {
    opaque: Vec<DrawFunction<M::View>>,
    transparent: Vec<DrawFunction<M::View>>,
}

impl<M: DrawPass> DrawFunctions<M> {
    pub fn new() -> Self {
        Self {
            opaque: Vec::new(),
            transparent: Vec::new(),
        }
    }

    pub fn add<D: Draw<View = M::View>>(&mut self) {
        const VIEW_GROUP: u32 = 0;
        const MATERIAL_GROUP: u32 = 1;
        const VERTEX_BUFFER_SLOT: u32 = 0;
        const INSTANCE_BUFFER_SLOT: u32 = 1;

        let pass: DrawFunction<M::View> = |ctx, view, view_buffer, meshes, state| {
            let draw_calls = ctx.world().resource::<ViewDrawCalls<D>>();
            let Some(draw_calls) = draw_calls.get(view.entity()) else {
                return;
            };

            let Some(pipeline_id) = ctx.world().try_resource::<DrawPipline<D>>() else {
                return;
            };

            let Some(pipeline) = ctx.get_render_pipeline(&pipeline_id) else {
                return;
            };

            let mesh_data = ctx.world().resource::<MeshDataBuffer<D::Mesh>>();
            let materials = ctx
                .world()
                .resource::<RenderAssets<MaterialBinding<D::Material>>>();

            state.set_pipeline(pipeline);
            state.set_vertex_buffer(INSTANCE_BUFFER_SLOT, mesh_data.buffer().slice(..));
            state.set_bind_group(
                VIEW_GROUP,
                view_buffer.bind_group(),
                &[view.dynamic_offset()],
            );

            let mut execute = |key: &BatchKey<D::Material>, batch: Range<u32>| {
                let mesh = match meshes.get(&(*key.mesh).into()) {
                    Some(mesh) => mesh,
                    None => return,
                };

                let material = match materials.get(&(*key.material).into()) {
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

                state.set_vertex_buffer(VERTEX_BUFFER_SLOT, mesh.vertex_buffer().slice(..));
                state.set_bind_group(MATERIAL_GROUP, material, &[]);

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

        match D::Material::mode() {
            BlendMode::Opaque => self.opaque.push(pass),
            BlendMode::Transparent => self.transparent.push(pass),
        }
    }

    pub fn draw(
        &self,
        ctx: &RenderContext,
        view: &RenderView<M::View>,
        buffer: &ViewBuffer<M::View>,
        meshes: &RenderAssets<RenderMesh>,
        state: &mut RenderState,
        mode: BlendMode,
    ) {
        match mode {
            BlendMode::Opaque => {
                for pass in &self.opaque {
                    pass(ctx, view, buffer, meshes, state);
                }
            }
            BlendMode::Transparent => {
                for pass in &self.transparent {
                    pass(ctx, view, buffer, meshes, state);
                }
            }
        }
    }
}

impl<M: DrawPass> Resource for DrawFunctions<M> {}

pub trait DrawPass: Send + Sync + 'static {
    type View: View;

    const NAME: super::Name;

    fn setup(builder: &mut super::PassBuilder) -> RenderPass;
}

pub type DrawCommand = fn(BlendMode, &mut RenderContext, &RenderAssets<RenderMesh>, &RenderPass);

pub struct DrawPassBuilder {
    setup: fn(&mut super::PassBuilder) -> RenderPass,
    command: DrawCommand,
}

impl DrawPassBuilder {
    pub fn new<P: DrawPass>() -> Self {
        Self {
            setup: P::setup,
            command: |mode, ctx, meshes, render_pass| {
                let view_buffer = ctx.world().resource::<ViewBuffer<P::View>>();
                let Some(view) = view_buffer.get_view(ctx.view()) else {
                    return;
                };

                let mut encoder = ctx.encoder();
                let passes = ctx.world().resource::<DrawFunctions<P>>();

                if let Some(mut render_pass) =
                    render_pass.begin(&mut encoder, ctx, view.clear_color())
                {
                    let mut state = RenderState::new(&mut render_pass);
                    passes.draw(ctx, view, view_buffer, meshes, &mut state, mode);
                }

                ctx.submit(encoder.finish());
            },
        }
    }

    pub fn build(self, builder: &mut super::PassBuilder) -> ErasedDrawPass {
        let pass = (self.setup)(builder);
        ErasedDrawPass {
            pass,
            command: self.command,
        }
    }
}

pub struct ErasedDrawPass {
    pass: RenderPass,
    command: DrawCommand,
}

impl ErasedDrawPass {
    pub fn execute(
        &self,
        mode: BlendMode,
        ctx: &mut RenderContext,
        meshes: &RenderAssets<RenderMesh>,
    ) {
        (self.command)(mode, ctx, meshes, &self.pass);
    }
}

pub struct MainDrawPass {
    builders: IndexMap<super::Name, DrawPassBuilder>,
}

impl MainDrawPass {
    pub const DEPTH_TEXTURE: super::Name = "depth";

    pub fn new() -> Self {
        Self {
            builders: IndexMap::new(),
        }
    }

    pub fn add<P: DrawPass>(&mut self) {
        self.builders.insert(P::NAME, DrawPassBuilder::new::<P>());
    }
}

impl Resource for MainDrawPass {}

impl GraphPass for MainDrawPass {
    type Data = Vec<ErasedDrawPass>;

    const NAME: super::Name = "MainDrawPass";

    fn setup(self, builder: &mut super::PassBuilder) -> Self::Data {
        builder.create::<TextureView>(
            Self::DEPTH_TEXTURE,
            TextureDesc {
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: RenderSurface::DEPTH_FORMAT,
            },
        );

        let mut commands = Vec::new();
        for command in self.builders.into_values() {
            commands.push(command.build(builder));
        }

        commands
    }

    fn execute(ctx: &mut RenderContext, data: &Self::Data) {
        let meshes = ctx.world().resource::<RenderAssets<RenderMesh>>();
        for pass in data {
            pass.execute(BlendMode::Opaque, ctx, meshes);
        }

        for pass in data {
            pass.execute(BlendMode::Transparent, ctx, meshes);
        }
    }
}

pub trait Draw: Send + Sync + 'static {
    type View: View;
    type Mesh: MeshData;
    type Material: Material;
    type Pass: DrawPass<View = Self::View>;
    type Query: BaseQuery;

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
