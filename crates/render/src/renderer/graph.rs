use crate::{
    ComputePipeline, PipelineCache, PipelineId, RenderDevice, RenderPipeline, RenderSurface,
};
use ecs::{Entity, IndexMap, NonSendMut, Res, Resource, world::World};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

pub type Name = &'static str;
pub type NodeId = u32;
pub type ResourceId = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Imported,
    Transient,
}

pub trait GraphResource: Any + Sized + 'static {
    type Desc: Any + 'static;

    const NAME: Name;

    fn create(
        device: &RenderDevice,
        surface: &RenderSurface,
        name: Name,
        desc: &Self::Desc,
    ) -> Self;
}

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

pub type CreateResource = fn(&RenderDevice, &RenderSurface, Name, &dyn Any) -> Box<dyn Any>;

pub struct ResourceEntry {
    pub id: ResourceId,
    pub name: Name,
    pub version: u32,
    pub ty: ResourceType,
    desc: Arc<dyn Any>,
    object: Option<Box<dyn Any>>,
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

    pub fn import<R: GraphResource>(id: ResourceId, object: Option<R>) -> Self {
        Self {
            id,
            name: R::NAME,
            version: 0,
            ty: ResourceType::Imported,
            desc: Arc::new(()),
            object: object.map(|o| Box::new(o) as Box<dyn Any>),
            creator: None,
            last_pass: None,
            create: |_, _, _, _| Box::new(()),
        }
    }

    pub fn create(&mut self, device: &RenderDevice, surface: &RenderSurface) {
        let object = (self.create)(device, surface, self.name, &self.desc);
        self.object = Some(object)
    }

    pub fn inc_version(&mut self) -> u32 {
        self.version += 1;
        self.version
    }

    pub fn destroy(&mut self) {
        self.object = None;
        self.last_pass = None;
        self.creator = None;
    }
}

pub type PassExecutor = Box<dyn Fn(&mut RenderContext)>;

pub struct PassNode {
    pub id: NodeId,
    pub name: Name,
    pub creates: Vec<NodeId>,
    pub reads: Vec<NodeId>,
    pub writes: Vec<NodeId>,
    pub has_side_effect: bool,
    executor: PassExecutor,
}

impl PassNode {
    pub fn execute(&self, ctx: &mut RenderContext) {
        (self.executor)(ctx);
    }
}

pub struct PassBuilder<'a> {
    id: NodeId,
    name: Name,
    creates: Vec<ResourceId>,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    has_side_effect: bool,
    graph: &'a mut RenderGraph,
}

impl<'a> PassBuilder<'a> {
    pub fn new(id: NodeId, name: Name, graph: &'a mut RenderGraph) -> Self {
        Self {
            id,
            name,
            creates: vec![],
            reads: vec![],
            writes: vec![],
            has_side_effect: false,
            graph,
        }
    }

    pub fn create<R: GraphResource>(&mut self, desc: R::Desc) -> ResourceId {
        let resource = self.graph.entries.len() as u32;
        let entry = ResourceEntry::new::<R>(resource, R::NAME, ResourceType::Transient, desc);
        let node = ResourceNode::new(self.graph.resources.len() as u32, resource);

        self.graph.resources.push(node);
        self.graph.entries.insert(TypeId::of::<R>(), entry);
        self.creates.push(resource);

        resource
    }

    pub fn read<G: GraphResource>(&mut self) -> ResourceId {
        let id = self
            .graph
            .get_resource_entry::<G>()
            .expect("resource not found")
            .id;

        self.reads.push(id);

        id
    }

    pub fn write<G: GraphResource>(&mut self) -> ResourceId {
        let entry = match self.graph.get_resource_entry::<G>() {
            Some(entry) => entry,
            None => {
                self.graph.import::<G>(None);
                self.graph
                    .get_resource_entry::<G>()
                    .expect("resource not found")
            }
        };

        let id = entry.id;

        if entry.ty == ResourceType::Imported {
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

    pub fn force(&mut self) {
        self.has_side_effect = true;
    }

    fn entry_mut(&mut self, id: ResourceId) -> &mut ResourceEntry {
        let node = &self.graph.resources[id as usize];
        &mut self.graph.entries[node.resource as usize]
    }

    fn build<P: RenderGraphPass>(mut self, pass: P) -> PassNode {
        let executor = P::setup(pass, &mut self);
        PassNode {
            id: self.id,
            name: self.name,
            creates: self.creates,
            reads: self.reads,
            writes: self.writes,
            has_side_effect: self.has_side_effect,
            executor: Box::new(executor),
        }
    }
}

pub trait RenderGraphPass {
    const NAME: Name;

    fn setup(self, builder: &mut PassBuilder) -> impl Fn(&mut RenderContext) + 'static;
}

pub trait SubGraph: Copy {
    const NAME: Name;

    fn name(&self) -> Name {
        Self::NAME
    }
}

pub struct SubgraphPass(pub Name);

impl RenderGraphPass for SubgraphPass {
    const NAME: Name = "SubgraphPass";

    fn setup(self, _: &mut PassBuilder) -> impl Fn(&mut RenderContext) + 'static {
        move |ctx| {
            ctx.run_sub_graph(self.0);
        }
    }
}

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

pub struct RenderGraph {
    passes: Vec<PassNode>,
    resources: Vec<ResourceNode>,
    entries: IndexMap<TypeId, ResourceEntry>,
    sub_graphs: HashMap<Name, RenderGraph>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            passes: vec![],
            resources: vec![],
            entries: IndexMap::new(),
            sub_graphs: HashMap::new(),
        }
    }

    pub fn add_pass<P: RenderGraphPass>(&mut self, pass: P) -> NodeId {
        let id = self.passes.len() as u32;
        let node = PassBuilder::new(id, P::NAME, self).build::<P>(pass);
        self.passes.push(node);

        id
    }

    pub fn add_sub_graph(&mut self, sub_graph: impl SubGraph) {
        let id = self.add_pass(SubgraphPass(sub_graph.name()));
        self.passes[id as usize].has_side_effect = true;
        self.passes[id as usize].name = sub_graph.name();

        self.sub_graphs.insert(sub_graph.name(), RenderGraph::new());
    }

    pub fn import<R: GraphResource>(&mut self, resource: Option<R>) {
        let id = self.entries.len() as u32;
        match self.entries.entry(TypeId::of::<R>()) {
            ecs::map::Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                if entry.ty == ResourceType::Transient {
                    panic!("transient resource already exists: {}", R::NAME);
                } else {
                    entry.object = resource.map(|r| Box::new(r) as Box<dyn Any>);
                }
            }
            ecs::map::Entry::Vacant(entry) => {
                let node = ResourceNode::new(self.resources.len() as u32, id);
                let resource = ResourceEntry::import::<R>(id, resource);

                self.resources.push(node);
                entry.insert(resource);
            }
        }
    }

    pub fn destroy<R: GraphResource>(&mut self) {
        if let Some(entry) = self.entries.get_mut(&TypeId::of::<R>()) {
            entry.destroy();
        }
    }

    pub fn get_sub_graph(&self, sub_graph: impl SubGraph) -> Option<&RenderGraph> {
        self.sub_graphs.get(sub_graph.name())
    }

    pub fn get_sub_graph_mut(&mut self, sub_graph: impl SubGraph) -> Option<&mut RenderGraph> {
        self.sub_graphs.get_mut(sub_graph.name())
    }

    pub fn get_resource<G: GraphResource>(&self, id: ResourceId) -> Option<&G> {
        let node = self.resources.get(id as usize)?;

        self.entries[node.resource as usize]
            .object
            .as_ref()?
            .downcast_ref::<G>()
    }

    pub fn get_resource_entry<G: GraphResource>(&self) -> Option<&ResourceEntry> {
        self.entries.get(&TypeId::of::<G>())
    }

    fn compile(&self) -> CompiledGraph {
        let mut passes = self
            .passes
            .iter()
            .map(|p| p.writes.len())
            .collect::<Vec<_>>();

        let mut resources = self
            .entries
            .values()
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
            if *ref_count == 0 && !self.passes[pass].has_side_effect {
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
        view: Option<Entity>,
    ) {
        let mut compiled = self.compile();

        for pass in &compiled.passes {
            for id in self.passes[*pass].creates.iter().copied() {
                let resource = self.resources[id as usize].resource;
                self.entries[resource as usize].create(device, surface);
            }

            {
                let mut sub_graphs = std::mem::take(&mut self.sub_graphs);
                let mut ctx =
                    RenderContext::new(self, &mut sub_graphs, world, device, surface, view);
                self.passes[*pass].execute(&mut ctx);
                device.queue.submit(ctx.finish());
                self.sub_graphs = sub_graphs;
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

    pub(crate) fn run_graph(
        mut graph: NonSendMut<RenderGraph>,
        device: Res<RenderDevice>,
        surface: Res<RenderSurface>,
        world: &World,
    ) {
        let Ok(surface_texture) = surface.texture() else {
            return;
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        graph.import::<RenderTarget>(Some(RenderTarget::new(view)));

        graph.run(world, &device, &surface, None);

        graph.destroy::<RenderTarget>();

        surface_texture.present();
    }
}

impl Resource for RenderGraph {}

pub struct RenderContext<'a> {
    view: Option<Entity>,
    graph: &'a RenderGraph,
    sub_graphs: &'a mut HashMap<Name, RenderGraph>,
    world: &'a World,
    device: &'a RenderDevice,
    surface: &'a RenderSurface,
    pipelines: &'a PipelineCache,
    buffers: Vec<wgpu::CommandBuffer>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        graph: &'a RenderGraph,
        sub_graphs: &'a mut HashMap<Name, RenderGraph>,
        world: &'a World,
        device: &'a RenderDevice,
        surface: &'a RenderSurface,
        view: Option<Entity>,
    ) -> Self {
        Self {
            view,
            graph,
            sub_graphs,
            world,
            device,
            surface,
            pipelines: world.resource::<PipelineCache>(),
            buffers: Vec::new(),
        }
    }

    pub fn view(&self) -> Option<Entity> {
        self.view
    }

    pub fn world(&self) -> &'a World {
        self.world
    }

    pub fn device(&self) -> &'a RenderDevice {
        self.device
    }

    pub fn surface(&self) -> &'a RenderSurface {
        self.surface
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

    pub fn set_view(&mut self, view: Option<Entity>) {
        self.view = view
    }

    pub(crate) fn run_sub_graph(&mut self, name: Name) {
        if let Some(graph) = self.sub_graphs.get_mut(name) {
            graph.run(self.world, self.device, self.surface, self.view);
        }
    }
}

pub struct TextureDesc {
    pub usage: wgpu::TextureUsages,
    pub format: wgpu::TextureFormat,
}

pub struct RenderTarget(wgpu::TextureView);
impl From<wgpu::TextureView> for RenderTarget {
    fn from(value: wgpu::TextureView) -> Self {
        Self(value)
    }
}

impl RenderTarget {
    pub fn new(view: wgpu::TextureView) -> Self {
        Self(view)
    }
}

impl std::ops::Deref for RenderTarget {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GraphResource for RenderTarget {
    type Desc = ();

    const NAME: Name = "RenderTarget";

    fn create(device: &RenderDevice, surface: &RenderSurface, name: Name, _: &Self::Desc) -> Self {
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
            format: surface.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self(texture.create_view(&wgpu::TextureViewDescriptor::default()))
    }
}
