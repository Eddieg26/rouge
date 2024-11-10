use super::{
    systems::{RunMode, SystemRunner},
    IntoSystemConfigs, System, SystemConfig,
};
use crate::{
    core::Type,
    system::{AccessType, WorldAccess, WorldAccessMeta},
    world::cell::WorldCell,
};
use indexmap::IndexMap;

pub trait Phase: Sized + 'static {
    fn id(&self) -> PhaseId {
        PhaseId::of::<Self>()
    }

    fn schedule() -> Schedule {
        Schedule::new(PhaseId::of::<Self>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseId(Type);

impl PhaseId {
    pub fn of<P: Phase>() -> Self {
        Self(Type::of::<P>())
    }

    pub fn dynamic(ty: u32) -> Self {
        Self(Type::dynamic(ty))
    }
}

impl Into<Type> for PhaseId {
    fn into(self) -> Type {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct SystemGroup {
    indexes: Vec<usize>,
    send: usize,
}

impl SystemGroup {
    pub fn new(indexes: Vec<usize>, send: usize) -> Self {
        Self { indexes, send }
    }

    pub fn indexes(&self) -> &[usize] {
        &self.indexes
    }

    pub fn send(&self) -> usize {
        self.send
    }
}

pub struct SystemGraph {
    systems: Vec<System>,
    groups: Vec<SystemGroup>,
}

impl SystemGraph {
    pub fn new(mode: RunMode, mut configs: Vec<SystemConfig>) -> Self {
        let (groups, systems) = match mode {
            RunMode::Sequential => {
                let systems = configs.drain(..).map(System::new).collect::<Vec<_>>();
                let indexes = (0..systems.len()).collect::<Vec<_>>();
                let group = SystemGroup::new(indexes, systems.len());
                (vec![group], systems)
            }
            RunMode::Parallel => {
                #[derive(Default)]
                struct GroupInfo {
                    send: Vec<usize>,
                    non_send: Vec<usize>,
                    access: IndexMap<Type, (bool, bool)>,
                }

                impl GroupInfo {
                    fn new_send(index: usize, access: Vec<WorldAccess>) -> Self {
                        let mut group = Self::default();
                        group.with_access(access);
                        group.send.push(index);
                        group
                    }

                    fn new_non_send(index: usize, access: Vec<WorldAccess>) -> Self {
                        let mut group = Self::default();
                        group.with_access(access);
                        group.non_send.push(index);
                        group
                    }

                    fn with_access(&mut self, access: Vec<WorldAccess>) {
                        for access in access {
                            let (ty, access, _) = access.access_ty();
                            let (read, write) = self.access.entry(ty).or_default();
                            *read |= access == AccessType::Read;
                            *write |= access == AccessType::Write;
                        }
                    }
                }

                let mut groups = Vec::<GroupInfo>::new();
                let mut systems = Vec::with_capacity(configs.len());

                for (index, config) in configs.drain(..).enumerate() {
                    let mut last_group_index: Option<usize> = None;
                    let access = config.access();
                    for (group_index, group) in groups.iter().enumerate().rev() {
                        let mut has_dependency = false;
                        for world_access in &access {
                            let WorldAccessMeta { ty, access, .. } = world_access.meta();
                            if group
                                .access
                                .get(&ty)
                                .is_some_and(|(_, write)| *write || access == AccessType::Write)
                            {
                                has_dependency = true;
                                break;
                            }
                        }

                        if !has_dependency {
                            last_group_index = Some(group_index);
                        }
                    }

                    match last_group_index {
                        Some(group) => {
                            match config.is_send {
                                true => groups[group].send.push(index),
                                false => groups[group].non_send.push(index),
                            }
                            groups[group].with_access(access);
                        }
                        None => {
                            let group = match config.is_send {
                                true => GroupInfo::new_send(index, access),
                                false => GroupInfo::new_non_send(index, access)
                            };

                            groups.push(group);
                        }
                    }

                    systems.push(System::new(config));
                }

                let groups = groups
                    .into_iter()
                    .map(|group| {
                        let mut indexes = group.send;
                        let send = indexes.len();
                        indexes.extend(group.non_send);
                        SystemGroup::new(indexes, send)
                    })
                    .collect();

                (groups, systems)
            }
        };

        Self { systems, groups }
    }

    pub fn systems(&self) -> &[System] {
        &self.systems
    }

    pub fn groups(&self) -> &[SystemGroup] {
        &self.groups
    }
}

pub struct PhaseSystemGraphs {
    graphs: IndexMap<PhaseId, SystemGraph>,
}

impl PhaseSystemGraphs {
    pub fn new() -> Self {
        Self {
            graphs: IndexMap::new(),
        }
    }

    pub fn add_graph(&mut self, phase: PhaseId, graph: SystemGraph) {
        self.graphs.insert(phase, graph);
    }

    pub fn get(&self, id: PhaseId) -> Option<&SystemGraph> {
        self.graphs.get(&id)
    }
}

#[derive(Default)]
pub struct PhaseSystemConfigs {
    configs: IndexMap<PhaseId, Vec<SystemConfig>>,
}

impl PhaseSystemConfigs {
    pub fn new() -> Self {
        Self {
            configs: IndexMap::new(),
        }
    }

    pub fn get(&self, id: &PhaseId) -> Option<&[SystemConfig]> {
        self.configs.get(id).map(AsRef::as_ref)
    }

    pub fn len(&self) -> usize {
        self.configs.len()
    }

    pub fn add_systems<M>(&mut self, phase: impl Phase, configs: impl IntoSystemConfigs<M>) {
        self.configs
            .entry(phase.id())
            .or_default()
            .extend(configs.configs());
    }

    pub fn into_graphs(mut self, mode: RunMode) -> PhaseSystemGraphs {
        let graphs = self
            .configs
            .drain(..)
            .map(|(id, configs)| (id, SystemGraph::new(mode, configs)));

        PhaseSystemGraphs {
            graphs: graphs.collect(),
        }
    }
}

pub struct RunContext<'a> {
    world: &'a WorldCell<'a>,
    systems: &'a [&'a SystemGraph],
    runner: &'a dyn SystemRunner,
}

impl<'a> RunContext<'a> {
    pub fn new(
        world: &'a WorldCell,
        systems: &'a [&'a SystemGraph],
        runner: &'a dyn SystemRunner,
    ) -> Self {
        Self {
            world,
            systems,
            runner,
        }
    }

    pub fn run(&self) {
        self.runner.run(self.world, self.systems);
    }
}

pub trait PhaseRunner: Send + 'static {
    fn run(&mut self, ctx: RunContext);
}

impl PhaseRunner for () {
    fn run(&mut self, ctx: RunContext) {
        ctx.run();
    }
}

pub struct PhaseRunners {
    default: Box<dyn PhaseRunner>,
    runners: IndexMap<PhaseId, Box<dyn PhaseRunner>>,
}

impl PhaseRunners {
    pub fn new() -> Self {
        Self {
            default: Box::new(()),
            runners: IndexMap::new(),
        }
    }

    pub fn add_runner(&mut self, id: PhaseId, runner: impl PhaseRunner) {
        self.runners.insert(id, Box::new(runner));
    }

    pub fn get(&self, id: &PhaseId) -> &dyn PhaseRunner {
        self.runners
            .get(id)
            .map(|r| r.as_ref())
            .unwrap_or(self.default.as_ref())
    }

    pub fn get_mut(&mut self, id: &PhaseId) -> &mut dyn PhaseRunner {
        self.runners
            .get_mut(id)
            .map(|r| r.as_mut())
            .unwrap_or(self.default.as_mut())
    }
}

impl Default for PhaseRunners {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Schedule {
    id: PhaseId,
    children: Vec<Schedule>,
}

impl Schedule {
    pub fn new(id: PhaseId) -> Self {
        Self {
            id,
            children: Vec::new(),
        }
    }

    pub fn from<P: Phase>() -> Self {
        Self::new(PhaseId::of::<P>())
    }

    pub fn id(&self) -> PhaseId {
        self.id
    }

    pub fn children(&self) -> &[Schedule] {
        &self.children
    }

    pub fn has_child(&self, phase: PhaseId, recursive: bool) -> bool {
        if self.children.iter().any(|c| c.id() == phase) {
            true
        } else if recursive {
            self.children.iter().any(|c| c.has_child(phase, recursive))
        } else {
            false
        }
    }

    pub fn add_child(&mut self, child: Schedule) {
        self.children.push(child);
    }

    pub fn add_sub_child<Main: Phase, Sub: Phase>(&mut self) -> bool {
        let main = PhaseId::of::<Main>();
        if let Some(index) = self.children.iter().position(|child| child.id == main) {
            self.children[index].add_child(Sub::schedule());
            true
        } else {
            self.children
                .iter_mut()
                .any(|schedule| schedule.add_sub_child::<Main, Sub>())
        }
    }

    pub fn insert_before<Main: Phase, Before: Phase>(&mut self) -> bool {
        let main = PhaseId::of::<Main>();
        if let Some(index) = self.children.iter().position(|child| child.id == main) {
            self.children.insert(index, Before::schedule());
            true
        } else {
            self.children
                .iter_mut()
                .any(|schedule| schedule.insert_before::<Main, Before>())
        }
    }

    pub fn insert_after<Main: Phase, After: Phase>(&mut self) -> bool {
        let main = PhaseId::of::<Main>();
        if let Some(index) = self.children.iter().position(|child| child.id == main) {
            self.children.insert(index + 1, After::schedule());
            true
        } else {
            self.children
                .iter_mut()
                .any(|schedule| schedule.insert_after::<Main, After>())
        }
    }

    pub fn child(&self, id: PhaseId, recursive: bool) -> Option<&Schedule> {
        if let Some(child) = self.children.iter().find(|c| c.id() == id) {
            Some(child)
        } else if recursive {
            self.children.iter().find_map(|c| c.child(id, recursive))
        } else {
            None
        }
    }
}
