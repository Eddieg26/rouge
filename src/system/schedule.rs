use super::{
    systems::{RunMode, SystemRunner},
    IntoSystemConfigs, System, SystemConfig,
};
use crate::{
    core::registry::Type,
    system::{AccessType, WorldAccess},
    world::World,
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

pub struct SystemGraph {
    systems: Vec<System>,
    order: Vec<Vec<usize>>,
}

impl SystemGraph {
    pub fn new(mode: RunMode, mut configs: Vec<SystemConfig>) -> Self {
        let (order, systems) = match mode {
            RunMode::Sequential => {
                let systems = configs.drain(..).map(System::new).collect::<Vec<_>>();
                let order = vec![systems.iter().enumerate().map(|(i, _)| i).collect()];
                (order, systems)
            }
            RunMode::Parallel => {
                #[derive(Default)]
                struct Group {
                    indexes: Vec<usize>,
                    access: IndexMap<Type, (bool, bool)>,
                    exclusive: bool,
                }

                impl Group {
                    fn new(index: usize, world_access: Vec<WorldAccess>) -> Self {
                        let mut group = Self {
                            indexes: vec![index],
                            access: IndexMap::new(),
                            exclusive: false,
                        };

                        group.with_access(world_access);
                        group
                    }

                    fn set_exclusive(mut self) -> Self {
                        self.exclusive = true;
                        self
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

                pub enum GroupType {
                    Exclusive(usize),
                    Shared(usize),
                }

                let mut groups = Vec::<Group>::new();
                let mut systems = Vec::with_capacity(configs.len());

                for (index, config) in configs.drain(..).enumerate() {
                    let mut last_valid_group: Option<GroupType> = None;
                    let access = config.access();
                    for (group_index, group) in groups.iter().enumerate().rev() {
                        if group.exclusive {
                            continue;
                        }

                        let mut group_ty = Some(GroupType::Shared(group_index));
                        for world_access in &access {
                            let (ty, access, exclusive) = world_access.access_ty();

                            if exclusive {
                                group_ty = Some(GroupType::Exclusive(group_index));
                                break;
                            }

                            if group
                                .access
                                .get(&ty)
                                .is_some_and(|(_, write)| *write || access == AccessType::Write)
                            {
                                group_ty = None;
                                break;
                            }
                        }

                        match group_ty {
                            Some(GroupType::Exclusive(index)) => {
                                last_valid_group = Some(GroupType::Exclusive(index));
                                break;
                            }
                            Some(GroupType::Shared(index)) => {
                                last_valid_group = Some(GroupType::Shared(index))
                            }
                            None => (),
                        }
                    }

                    match last_valid_group {
                        Some(GroupType::Exclusive(group)) => {
                            groups.insert(group, Group::new(index, access).set_exclusive());
                        }
                        Some(GroupType::Shared(group)) => {
                            groups[group].indexes.push(index);
                            groups[group].with_access(access);
                        }
                        None => groups.push(Group::new(index, access)),
                    }

                    systems.push(config.into());
                }

                let order = groups.drain(..).map(|group| group.indexes).collect();
                (order, systems)
            }
        };

        Self { systems, order }
    }

    pub fn systems(&self) -> &[System] {
        &self.systems
    }

    pub fn order(&self) -> &[Vec<usize>] {
        &self.order
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
    world: &'a World,
    systems: &'a [&'a SystemGraph],
    runner: &'a dyn SystemRunner,
}

impl<'a> RunContext<'a> {
    pub fn new(
        world: &'a World,
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
