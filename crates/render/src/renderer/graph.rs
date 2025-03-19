use crate::{device::RenderDevice, surface::RenderSurface};
use std::{any::Any, sync::Arc};

pub trait GraphResource: Sized + Any + Send + Sync + 'static {
    type Desc: Any + Send + Sync + 'static;

    fn create(device: &RenderDevice, surface: &RenderSurface, desc: &Self::Desc) -> Self;
}

pub type Name = &'static str;
pub type NodeId = u32;
pub type ResourceId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Transient,
    Imported,
}

#[derive(Debug, Clone)]
pub struct ResourceNode {
    id: ResourceId,
    name: Name,
    version: u32,
    ref_count: u32,
    kind: ResourceKind,
    desc: Arc<dyn Any>,
    create: fn(&RenderDevice, &RenderSurface, &dyn Any) -> Box<dyn Any>,
    creator: Option<NodeId>,
    last_user: Option<NodeId>,
}

impl ResourceNode {
    pub fn create(&self, device: &RenderDevice, surface: &RenderSurface) -> Box<dyn Any> {
        (self.create)(device, surface, self.desc.as_ref())
    }
}

pub struct ResourceEntry {
    id: ResourceId,
    version: u32,
    resource: Option<Box<dyn Any>>,
}

pub trait GraphPass {
    type Data: Any + Send + Sync + 'static;
    const NAME: Name;

    fn setup(builder: &mut PassBuilder) -> Self::Data;
    fn execute(data: &Self::Data, ctx: &RenderContext);
}

pub type PassData = Box<dyn Any>;

pub type Executor = Box<dyn Fn(&PassData, &RenderContext)>;

pub struct PassNode {
    id: NodeId,
    name: Name,
    creates: Vec<ResourceId>,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    has_side_effect: bool,
    ref_count: u32,
    data: Box<dyn Any>,
    executor: Executor,
}

impl PassNode {
    pub fn execute(&self, data: &PassData, ctx: &RenderContext) {
        (self.executor)(data, ctx);
    }
}

pub struct RenderGraph {
    passes: Vec<PassNode>,
    resources: Vec<ResourceNode>,
    entries: Vec<ResourceEntry>,
}

impl RenderGraph {
    pub fn add_pass<G: GraphPass>(&mut self) -> NodeId {
        let id = self.passes.len() as NodeId;
        let pass = PassBuilder::new(self, id).build::<G>();

        self.passes.push(pass);

        id
    }

    pub fn import<R: GraphResource>(&mut self, name: Name, resource: Option<R>) -> ResourceId {
        // TODO Update resource if it already exists
        if let Some(id) = self.resources.iter().find(|r| r.name == name).map(|r| r.id) {
            return id;
        }

        let id = self.resources.len() as ResourceId;

        self.resources.push(ResourceNode {
            id,
            name,
            version: 0,
            ref_count: 0,
            kind: ResourceKind::Imported,
            desc: Arc::new(()),
            create: |_, _, _| unreachable!("Imported resources cannot be created"),
            creator: None,
            last_user: None,
        });

        self.entries.push(ResourceEntry {
            id,
            version: 0,
            resource: resource.map(|r| Box::new(r) as Box<dyn Any>),
        });

        id
    }

    pub fn remove(&mut self, id: ResourceId) {
        self.resource_mut(id).creator = None;
        self.entry_mut(id).resource = None;
    }

    pub fn compile(&mut self) -> Vec<NodeId> {
        for index in 0..self.passes.len() {
            self.pass_mut(index).ref_count = self.pass(index).writes.len() as u32;
            for read in 0..self.pass(index).reads.len() {
                let id = self.pass(index).reads[read];
                self.resource_mut(id).ref_count += 1;
            }

            for write in 0..self.passes[index].writes.len() {
                let id = self.passes[index].writes[write];
                self.resource_mut(id).creator = Some(index as NodeId);
            }
        }

        let mut unreferenced = self
            .resources
            .iter()
            .filter_map(|r| (r.ref_count == 0).then_some(r.id))
            .collect::<Vec<_>>();

        while let Some(resource) = unreferenced.pop() {
            let Some(node) = self.resource(resource).creator.map(|n| n as usize) else {
                continue;
            };

            if self.pass(node).has_side_effect {
                continue;
            }

            assert!(self.pass(node).ref_count >= 1);
            self.pass_mut(node).ref_count -= 1;

            if self.pass(node).ref_count == 0 {
                for index in 0..self.pass(node).reads.len() {
                    let id = self.pass(node).reads[index];
                    self.resource_mut(id).ref_count -= 1;
                    if self.resource(id).ref_count == 0 {
                        unreferenced.push(id);
                    }
                }
            }
        }

        let mut passes = vec![];
        for node in 0..self.passes.len() {
            if self.pass(node).ref_count == 0 {
                continue;
            }

            for create in 0..self.pass(node).creates.len() {
                let id = self.pass(node).creates[create];
                self.resource_mut(id).creator = Some(node as NodeId);
            }

            for write in 0..self.pass(node).writes.len() {
                let id = self.pass(node).writes[write];
                self.resource_mut(id).creator = Some(node as NodeId);
            }

            for read in 0..self.pass(node).reads.len() {
                let id = self.pass(node).reads[read];
                self.resource_mut(id).creator = Some(node as NodeId);
            }

            passes.push(node as u32);
        }

        passes
    }

    pub fn run(&mut self, device: &RenderDevice, surface: &RenderSurface, nodes: Vec<NodeId>) {
        for node in nodes {
            for index in 0..self.passes[node as usize].creates.len() {
                let id = self.passes[node as usize].creates[index];
                let resource = self.resource(id).create(device, surface);

                self.entry_mut(id).resource = Some(resource);
            }

            self.execute_node(device, node);

            for entry in 0..self.entries.len() {
                if {
                    let resource = &self.resources[self.entries[entry].id as usize];
                    resource.last_user == Some(node) && resource.kind == ResourceKind::Transient
                } {
                    self.entries[entry].resource = None;
                }
            }
        }
    }

    fn execute_node(&self, device: &RenderDevice, node: NodeId) {
        let context = RenderContext {
            graph: self,
            device,
        };

        let data = &self.pass(node as usize).data;
        self.pass(node as usize).execute(data, &context);
    }

    fn resource(&self, id: ResourceId) -> &ResourceNode {
        assert!(id < self.resources.len() as ResourceId);
        &self.resources[id as usize]
    }

    fn resource_mut(&mut self, id: ResourceId) -> &mut ResourceNode {
        assert!(id < self.resources.len() as ResourceId);
        &mut self.resources[id as usize]
    }

    fn entry(&self, id: ResourceId) -> &ResourceEntry {
        let id = self.resource(id).id;
        assert!(id < self.entries.len() as ResourceId);
        &self.entries[id as usize]
    }

    fn entry_mut(&mut self, id: ResourceId) -> &mut ResourceEntry {
        let id = self.resource(id).id;
        assert!(id < self.entries.len() as ResourceId);
        &mut self.entries[id as usize]
    }

    fn pass(&self, index: usize) -> &PassNode {
        assert!(index < self.passes.len());
        &self.passes[index]
    }

    fn pass_mut(&mut self, index: usize) -> &mut PassNode {
        assert!(index < self.passes.len());
        &mut self.passes[index]
    }
}

pub struct RenderContext<'a> {
    graph: &'a RenderGraph,
    device: &'a RenderDevice,
}

impl<'a> RenderContext<'a> {
    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn get<R: GraphResource>(&self, id: ResourceId) -> &R {
        let entry = &self.graph.entries[id as usize];
        let resource = entry.resource.as_ref().unwrap();
        resource.downcast_ref::<R>().unwrap()
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }
}

pub struct PassBuilder<'a> {
    graph: &'a mut RenderGraph,
    id: NodeId,
    creates: Vec<ResourceId>,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    has_side_effect: bool,
}

impl<'a> PassBuilder<'a> {
    pub fn new(graph: &'a mut RenderGraph, id: NodeId) -> Self {
        Self {
            graph,
            id,
            creates: vec![],
            reads: vec![],
            writes: vec![],
            has_side_effect: false,
        }
    }

    fn build<G: GraphPass>(mut self) -> PassNode {
        let data = G::setup(&mut self);
        PassNode {
            id: self.id,
            name: G::NAME,
            creates: self.creates,
            reads: self.reads,
            writes: self.writes,
            has_side_effect: self.has_side_effect,
            ref_count: 0,
            data: Box::new(data),
            executor: Box::new(move |data, ctx| {
                let data = data.downcast_ref::<G::Data>().unwrap();
                G::execute(data, ctx);
            }),
        }
    }

    pub fn is_valid(&self, id: ResourceId) -> bool {
        assert!(id < self.graph.resources.len() as ResourceId);
        assert!(id < self.graph.entries.len() as ResourceId);

        let node = &self.graph.resources[id as usize];
        let entry = &self.graph.entries[id as usize];

        node.version == entry.version
    }

    pub fn create<R: GraphResource>(&mut self, name: Name, resource: R::Desc) -> ResourceId {
        let id = self.graph.resources.len() as ResourceId;

        self.graph.resources.push(ResourceNode {
            id,
            name,
            version: 0,
            ref_count: 0,
            kind: ResourceKind::Transient,
            desc: Arc::new(resource),
            create: |device, surface, desc| {
                let desc = desc.downcast_ref::<R::Desc>().unwrap();
                let resource = R::create(device, surface, desc);
                Box::new(resource) as Box<dyn Any>
            },
            creator: Some(self.id),
            last_user: None,
        });

        self.pass_mut().creates.push(id);
        self.pass_mut().writes.push(id);

        id
    }

    pub fn read(&mut self, resource: ResourceId) -> ResourceId {
        assert!(self.is_valid(resource));
        self.pass_mut().reads.push(resource);
        resource
    }

    pub fn write(&mut self, resource: ResourceId) -> ResourceId {
        assert!(self.is_valid(resource));

        if self.graph.resource(resource).kind == ResourceKind::Imported {
            self.has_side_effect = true;
        }

        if self.pass().creates.contains(&resource) {
            resource
        } else {
            self.pass_mut().reads.push(resource);

            let resource = self.write_resource(resource);
            self.pass_mut().writes.push(resource);
            resource
        }
    }

    fn write_resource(&mut self, id: ResourceId) -> ResourceId {
        let mut node = self.graph.resource(id).clone();
        self.graph.entry_mut(id).version += 1;

        node.id = self.graph.resources.len() as ResourceId;
        node.version = self.graph.entry(id).version;

        let id = node.id;
        self.graph.resources.push(node);

        id
    }

    fn pass(&self) -> &PassNode {
        assert!(self.id < self.graph.passes.len() as NodeId);
        &self.graph.passes[self.id as usize]
    }

    fn pass_mut(&mut self) -> &mut PassNode {
        assert!(self.id < self.graph.passes.len() as NodeId);
        &mut self.graph.passes[self.id as usize]
    }
}
