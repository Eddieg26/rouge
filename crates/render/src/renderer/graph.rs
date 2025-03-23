use super::{RenderView, View, ViewBuffer, ViewEntities};
use crate::{
    device::RenderDevice,
    resources::{Buffer, ComputePipeline, PipelineCache, PipelineId, RenderPipeline},
    surface::RenderSurface,
};
use ecs::{Entity, NonSendMut, Res, ResMut, Resource, world::World};
use std::{any::Any, sync::Arc};
use wgpu::{BufferSize, BufferUsages, TextureFormat, TextureUsages};

pub type Name = &'static str;

pub trait GraphResource: Any + Sized + 'static {
    type Desc: Any + 'static;

    fn create(
        device: &RenderDevice,
        surface: &RenderSurface,
        name: Name,
        desc: &Self::Desc,
    ) -> Self;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TextureDesc {
    pub usage: TextureUsages,
    pub format: TextureFormat,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            format: TextureFormat::Bgra8UnormSrgb,
        }
    }
}

pub struct TextureView(wgpu::TextureView);
impl std::ops::Deref for TextureView {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<wgpu::TextureView> for TextureView {
    fn as_ref(&self) -> &wgpu::TextureView {
        &self.0
    }
}

impl From<wgpu::TextureView> for TextureView {
    fn from(value: wgpu::TextureView) -> Self {
        Self(value)
    }
}

impl From<wgpu::Texture> for TextureView {
    fn from(value: wgpu::Texture) -> Self {
        Self(value.create_view(&Default::default()))
    }
}

impl GraphResource for TextureView {
    type Desc = TextureDesc;

    fn create(
        device: &RenderDevice,
        surface: &RenderSurface,
        name: Name,
        desc: &Self::Desc,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size: wgpu::Extent3d {
                width: surface.width(),
                height: surface.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: desc.usage,
            view_formats: &[desc.format],
        });

        texture.create_view(&Default::default()).into()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BufferDesc {
    pub size: BufferSize,
    pub usage: BufferUsages,
}

impl GraphResource for Buffer {
    type Desc = BufferDesc;

    fn create(device: &RenderDevice, _: &RenderSurface, name: Name, desc: &Self::Desc) -> Self {
        Buffer::new(device, desc.size.get(), desc.usage, Some(name.into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Imported,
    Transient,
}

pub type NodeId = u32;
pub type ResourceId = u32;
pub type ResourceDesc = Arc<dyn Any>;
pub type ResourceObj = Box<dyn Any>;
pub type CreateResource = fn(&RenderDevice, &RenderSurface, Name, &ResourceDesc) -> ResourceObj;

#[derive(Debug, Clone)]
pub struct ResourceNode {
    pub id: NodeId,
    pub resource: ResourceId,
    pub version: u32,
}

impl ResourceNode {
    pub fn new(id: NodeId, resource: ResourceId) -> Self {
        Self {
            id,
            resource,
            version: 0,
        }
    }
}

pub struct ResourceEntry {
    pub id: ResourceId,
    pub name: Name,
    pub version: u32,
    pub ty: ResourceType,
    desc: ResourceDesc,
    object: Option<ResourceObj>,
    creator: Option<NodeId>,
    last_pass: Option<NodeId>,
    create: CreateResource,
}

impl ResourceEntry {
    pub fn new<R: GraphResource>(
        id: ResourceId,
        name: Name,
        ty: ResourceType,
        desc: R::Desc,
    ) -> Self {
        Self {
            id,
            name,
            version: 0,
            ty,
            desc: Arc::new(desc),
            object: None,
            creator: None,
            last_pass: None,
            create: |device, surface, name, desc| {
                let desc = desc.downcast_ref::<R::Desc>().unwrap();
                let resource = R::create(device, surface, name, desc);
                Box::new(resource)
            },
        }
    }

    pub fn import<R: GraphResource>(id: ResourceId, name: Name, object: Option<R>) -> Self {
        Self {
            id,
            name,
            version: 0,
            ty: ResourceType::Imported,
            desc: Arc::new(()),
            object: object.map(|o| Box::new(o) as ResourceObj),
            creator: None,
            last_pass: None,
            create: |_, _, _, _| Box::new(()),
        }
    }

    fn surface() -> Self {
        Self {
            id: RenderGraph::SURFACE_ID,
            name: RenderGraph::SURFACE,
            version: 0,
            ty: ResourceType::Imported,
            desc: Arc::new(()),
            object: None,
            creator: None,
            last_pass: None,
            create: |_, _, _, _| Box::new(()),
        }
    }

    fn depth_texture() -> Self {
        Self::new::<TextureView>(
            RenderGraph::DEPTH_TEXTURE_ID,
            RenderGraph::DEPTH_TEXTURE,
            ResourceType::Transient,
            TextureDesc {
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: RenderSurface::DEFAULT_FORMAT,
            },
        )
    }

    pub fn create(&mut self, device: &RenderDevice, surface: &RenderSurface) {
        let object = (self.create)(device, surface, self.name, &self.desc);
        self.object = Some(object)
    }

    pub fn destroy(&mut self) {
        self.object = None;
        self.last_pass = None;
        self.creator = None;
    }
}

impl ResourceEntry {
    pub fn inc_version(&mut self) -> u32 {
        self.version += 1;
        self.version
    }
}

pub trait GraphPass: 'static {
    type Data: Any + 'static;

    const NAME: Name;

    fn setup(builder: &mut PassBuilder) -> Self::Data;
    fn execute(ctx: &mut RenderContext, data: &Self::Data);
}

pub type PassData = Box<dyn Any>;
pub type ExecutePass = fn(&mut RenderContext, &PassData);

pub struct ResourceInfo {
    id: ResourceId,
    ref_count: u32,
    creator: Option<NodeId>,
    last_pass: Option<NodeId>,
}

impl From<&ResourceEntry> for ResourceInfo {
    fn from(entry: &ResourceEntry) -> Self {
        Self {
            id: entry.id,
            ref_count: 0,
            creator: entry.creator,
            last_pass: entry.last_pass,
        }
    }
}

pub struct CompiledGraph {
    passes: Vec<usize>,
    resources: Vec<ResourceInfo>,
}

pub struct PassNode {
    id: NodeId,
    name: Name,
    data: PassData,
    creates: Vec<NodeId>,
    reads: Vec<NodeId>,
    writes: Vec<NodeId>,
    has_side_effect: bool,
    execute: ExecutePass,
}

impl PassNode {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn name(&self) -> Name {
        self.name
    }

    pub fn data(&self) -> &PassData {
        &self.data
    }

    pub fn reads(&self) -> &[NodeId] {
        &self.reads
    }

    pub fn writes(&self) -> &[NodeId] {
        &self.writes
    }

    pub fn has_side_effect(&self) -> bool {
        self.has_side_effect
    }

    pub fn execute(&self, ctx: &mut RenderContext) {
        (self.execute)(ctx, &self.data);
    }
}

pub struct RenderGraph {
    passes: Vec<PassNode>,
    resources: Vec<ResourceNode>,
    entries: Vec<ResourceEntry>,
}

impl RenderGraph {
    pub const SURFACE: Name = "Surface";
    const SURFACE_ID: ResourceId = 0;

    pub const DEPTH_TEXTURE: Name = "DepthTexture";
    const DEPTH_TEXTURE_ID: ResourceId = 1;

    pub fn new() -> Self {
        let resources = vec![
            ResourceNode::new(Self::SURFACE_ID, Self::SURFACE_ID),
            ResourceNode::new(Self::DEPTH_TEXTURE_ID, Self::DEPTH_TEXTURE_ID),
        ];
        let entries = vec![ResourceEntry::surface(), ResourceEntry::depth_texture()];

        Self {
            passes: vec![],
            resources,
            entries,
        }
    }

    pub fn add_pass<P: GraphPass>(&mut self) -> &mut Self {
        if self.passes.iter().any(|p| p.name() == P::NAME) {
            return self;
        }

        let id = self.passes.len() as u32;

        let node = PassBuilder::new(id, self).build::<P>();
        self.passes.push(node);

        self
    }

    pub fn import<R: GraphResource>(&mut self, name: Name, object: Option<R>) -> ResourceId {
        if let Some(resource) = self.entries.iter().position(|r| r.name == name) {
            self.entries[resource].object = object.map(|o| Box::new(o) as ResourceObj);
            self.resources
                .iter()
                .rev()
                .find(|n| n.resource == resource as u32)
                .unwrap()
                .id
        } else {
            let resource = self.entries.len() as u32;
            let entry = ResourceEntry::import(resource, name, object);
            let node = ResourceNode::new(self.resources.len() as u32, resource);

            self.resources.push(node);
            self.entries.push(entry);

            self.resources.len() as u32 - 1
        }
    }

    pub fn remove(&mut self, name: Name) {
        if let Some(resource) = self.entries.iter().find(|r| r.name == name) {
            let resource = self
                .resources
                .iter()
                .rev()
                .find(|n| n.resource == resource.id)
                .unwrap()
                .resource;

            self.entries[resource as usize].destroy();
        }
    }

    pub fn get_resource_id(&self, name: Name) -> Option<ResourceId> {
        self.entries.iter().position(|r| r.name == name).map(|id| {
            self.resources
                .iter()
                .rev()
                .find(|n| n.resource == id as u32)
                .unwrap()
                .id
        })
    }

    pub fn get_resource<R: GraphResource>(&self, id: ResourceId) -> Option<&R> {
        let resource = self.resources.get(id as usize)?;
        let entry = self.entries.get(resource.resource as usize)?;

        entry.object.as_ref().and_then(|o| o.downcast_ref::<R>())
    }

    pub fn surface_id(&self) -> ResourceId {
        self.resources
            .iter()
            .rev()
            .find_map(|node| (node.resource == Self::SURFACE_ID).then_some(node.resource))
            .expect("Surface resource not found") as u32
    }

    pub fn depth_texture_id(&self) -> ResourceId {
        self.resources
            .iter()
            .rev()
            .find_map(|node| (node.resource == Self::SURFACE_ID).then_some(node.resource))
            .expect("Depth texture resource not found") as u32
    }

    fn compile(&self) -> CompiledGraph {
        let mut passes = self
            .passes
            .iter()
            .map(|p| p.writes.len())
            .collect::<Vec<_>>();

        let mut resources = self
            .entries
            .iter()
            .map(ResourceInfo::from)
            .collect::<Vec<_>>();

        for pass in &self.passes {
            for id in &pass.reads {
                let resource = self.resources[*id as usize].resource;
                resources[resource as usize].ref_count += 1;
            }

            for id in &pass.writes {
                let resource = self.resources[*id as usize].resource;
                resources[resource as usize].creator = Some(pass.id);
            }
        }

        let mut unreferenced = resources
            .iter()
            .enumerate()
            .filter_map(|(id, info)| (info.ref_count == 0).then_some(id as u32))
            .collect::<Vec<_>>();

        while let Some(id) = unreferenced.pop() {
            let Some(pass) = resources[id as usize].creator else {
                continue;
            };

            if self.passes[pass as usize].has_side_effect {
                continue;
            }

            assert!(passes[pass as usize] >= 1);
            passes[pass as usize] -= 1;
            if passes[pass as usize] == 0 {
                for id in &self.passes[pass as usize].reads {
                    resources[*id as usize].ref_count -= 1;
                    if resources[*id as usize].ref_count == 0 {
                        unreferenced.push(*id);
                    }
                }
            }
        }

        let queue = passes.iter().enumerate().filter_map(|(pass, ref_count)| {
            if *ref_count == 0 {
                return None;
            }

            for id in &self.passes[pass].creates {
                let resource = self.resources[*id as usize].resource;
                resources[resource as usize].creator = Some(self.passes[pass].id);
            }

            for id in &self.passes[pass].reads {
                let resource = self.resources[*id as usize].resource;
                resources[resource as usize].last_pass = Some(self.passes[pass].id);
            }

            for id in &self.passes[pass].writes {
                let resource = self.resources[*id as usize].resource;
                resources[resource as usize].last_pass = Some(self.passes[pass].id);
            }

            Some(pass)
        });

        CompiledGraph {
            passes: queue.collect(),
            resources,
        }
    }

    pub fn run(
        &mut self,
        world: &World,
        device: &RenderDevice,
        surface: &RenderSurface,
        views: &mut ViewEntities,
    ) {
        let mut compiled = self.compile();

        for view in views.0.drain(..) {
            for pass in &compiled.passes {
                for id in self.passes[*pass].creates.iter().copied() {
                    let resource = self.resources[id as usize].resource;
                    self.entries[resource as usize].create(device, surface);
                }

                {
                    let mut ctx = RenderContext::new(view, self, world, device);
                    self.passes[*pass].execute(&mut ctx);
                    device.queue.submit(ctx.finish());
                }

                compiled.resources.iter_mut().for_each(|info| {
                    let destroy = info.last_pass == Some(self.passes[*pass].id)
                        && self.entries[info.id as usize].ty == ResourceType::Transient;
                    if destroy {
                        self.entries[info.id as usize].destroy();
                    }
                });
            }
        }
    }

    pub(crate) fn run_graph(
        world: &World,
        device: Res<RenderDevice>,
        surface: Res<RenderSurface>,
        mut graph: NonSendMut<RenderGraph>,
        mut views: ResMut<ViewEntities>,
    ) {
        let Ok(texture) = surface.texture() else {
            return;
        };

        let view = texture.texture.create_view(&Default::default());

        graph.import::<TextureView>("Surface", Some(view.into()));
        graph.run(world, &device, &surface, &mut views);
        graph.remove("Surface");

        texture.present();
    }
}

impl Resource for RenderGraph {}

pub struct PassBuilder<'a> {
    id: NodeId,
    creates: Vec<ResourceId>,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    has_side_effect: bool,
    graph: &'a mut RenderGraph,
}

impl<'a> PassBuilder<'a> {
    pub fn new(id: NodeId, graph: &'a mut RenderGraph) -> Self {
        Self {
            id,
            creates: vec![],
            reads: vec![],
            writes: vec![],
            has_side_effect: false,
            graph,
        }
    }

    pub fn create<R: GraphResource>(&mut self, name: Name, desc: R::Desc) -> ResourceId {
        let resource = self.graph.entries.len() as u32;
        let entry = ResourceEntry::new::<R>(resource, name, ResourceType::Transient, desc);
        let node = ResourceNode::new(self.graph.resources.len() as u32, resource);

        self.graph.resources.push(node);
        self.graph.entries.push(entry);
        self.creates.push(resource);

        resource
    }

    pub fn read(&mut self, id: ResourceId) -> ResourceId {
        assert!(self.validate(id));

        self.reads.push(id);

        id
    }

    pub fn write(&mut self, id: ResourceId) -> ResourceId {
        assert!(self.validate(id));

        if self.entry(id).ty == ResourceType::Imported {
            self.has_side_effect = true;
        }

        if self.creates.contains(&id) {
            self.writes.push(id);
            id
        } else {
            self.reads.push(id);

            let mut node = self.graph.resources[id as usize].clone();
            node.version = self.entry_mut(id).inc_version();

            let id = self.graph.resources.len() as u32;
            node.id = id;

            self.writes.push(id);
            self.graph.resources.push(node);

            id
        }
    }

    pub fn surface_id(&self) -> ResourceId {
        self.graph.surface_id()
    }

    fn validate(&self, id: ResourceId) -> bool {
        let index = id as usize;
        let node = &self.graph.resources[index];
        node.version == self.graph.entries[node.resource as usize].version
    }

    fn entry(&self, id: ResourceId) -> &ResourceEntry {
        let node = &self.graph.resources[id as usize];
        &self.graph.entries[node.resource as usize]
    }

    fn entry_mut(&mut self, id: ResourceId) -> &mut ResourceEntry {
        let node = &self.graph.resources[id as usize];
        &mut self.graph.entries[node.resource as usize]
    }

    fn build<P: GraphPass>(mut self) -> PassNode {
        let data = P::setup(&mut self);
        PassNode {
            id: self.id,
            name: P::NAME,
            data: Box::new(data),
            creates: self.creates,
            reads: self.reads,
            writes: self.writes,
            has_side_effect: self.has_side_effect,
            execute: |ctx, data| {
                let data = data.downcast_ref::<P::Data>().unwrap();
                P::execute(ctx, data);
            },
        }
    }
}

pub struct RenderContext<'a> {
    view: Entity,
    graph: &'a RenderGraph,
    world: &'a World,
    device: &'a RenderDevice,
    pipelines: &'a PipelineCache,
    buffers: Vec<wgpu::CommandBuffer>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        view: Entity,
        graph: &'a RenderGraph,
        world: &'a World,
        device: &'a RenderDevice,
    ) -> Self {
        Self {
            view,
            graph,
            world,
            device,
            pipelines: world.resource::<PipelineCache>(),
            buffers: Vec::new(),
        }
    }

    pub fn view(&self) -> Entity {
        self.view
    }

    pub fn render_view<V: View>(&self) -> Option<&RenderView<V>> {
        self.world.resource::<ViewBuffer<V>>().get_view(self.view)
    }

    pub fn world(&self) -> &'a World {
        self.world
    }

    pub fn device(&self) -> &'a RenderDevice {
        self.device
    }

    pub fn get_render_pipeline(&self, id: &PipelineId) -> Option<&RenderPipeline> {
        self.pipelines.get_render_pipeline(id)
    }

    pub fn get_compute_pipeline(&self, id: &PipelineId) -> Option<&ComputePipeline> {
        self.pipelines.get_compute_pipeline(id)
    }

    pub fn get<R: GraphResource>(&self, id: ResourceId) -> &R {
        self.graph
            .get_resource::<R>(id)
            .expect("resource not found")
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }

    pub fn submit(&mut self, buffer: wgpu::CommandBuffer) {
        self.buffers.push(buffer);
    }

    pub fn finish(self) -> Vec<wgpu::CommandBuffer> {
        self.buffers
    }
}
