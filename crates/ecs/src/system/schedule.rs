use std::any::TypeId;

use super::{IntoSystemConfigs, RunMode, SystemConfig, SystemGraph};
use crate::world::cell::WorldCell;
use hashbrown::HashMap;
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
pub struct PhaseId(TypeId);

impl PhaseId {
    pub fn of<P: Phase>() -> Self {
        Self(TypeId::of::<P>())
    }

    pub fn dynamic(ty: u128) -> Self {
        unsafe { Self(std::mem::transmute(ty)) }
    }
}

impl Into<TypeId> for PhaseId {
    fn into(self) -> TypeId {
        self.0
    }
}

pub trait SystemRunner: Send + Sync + 'static {
    fn run(&self, world: &WorldCell, systems: &[&SystemGraph]);
}

pub struct RunContext<'a> {
    world: &'a WorldCell<'a>,
    systems: &'a [&'a SystemGraph],
    mode: RunMode,
}

impl<'a> RunContext<'a> {
    pub fn new(world: &'a WorldCell, systems: &'a [&'a SystemGraph], mode: RunMode) -> Self {
        Self {
            world,
            systems,
            mode,
        }
    }

    pub fn run(&self) {
        for system in self.systems {
            system.run(self.world.get(), self.mode);
        }
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

    pub fn run(&self, world: &WorldCell, systems: &Systems, mode: RunMode) {
        let runner = systems.runner(self.id());
        let graphs = systems.get_phases(self.id());
        if !graphs.is_empty() {
            let ctx = RunContext::new(world, &graphs, mode);
            runner(ctx);
        }

        world.get_mut().flush(Some(self.id()));

        for child in self.children() {
            child.run(world, systems, mode);
        }
    }

    pub fn run_child(&self, child: PhaseId, world: &WorldCell, systems: &Systems, mode: RunMode) {
        if let Some(child) = self.child(child, true) {
            child.run(world, systems, mode);
        }
    }
}

pub struct Phases(HashMap<PhaseId, SystemGraph>);

impl Phases {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add_systems(&mut self, phase: PhaseId, configs: Vec<SystemConfig>) {
        self.0.insert(phase, SystemGraph::new(configs));
    }
}

pub struct Root;
impl Phase for Root {}

pub struct Systems {
    mode: RunMode,
    schedule: Schedule,
    phases: IndexMap<&'static str, Phases>,
    runners: HashMap<PhaseId, fn(RunContext)>,
    configs: HashMap<PhaseId, Vec<SystemConfig>>,
}

impl Systems {
    pub fn new(mode: RunMode) -> Self {
        Self {
            mode,
            schedule: Schedule::new(PhaseId::of::<Root>()),
            phases: IndexMap::new(),
            runners: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn mode(&self) -> RunMode {
        self.mode
    }

    pub fn add_phases(&mut self, name: &'static str, phases: Phases) {
        self.phases.insert(name, phases);
    }

    pub fn add_systems<M>(&mut self, phase: PhaseId, configs: impl IntoSystemConfigs<M>) {
        self.configs
            .entry(phase)
            .or_default()
            .extend(configs.configs());
    }

    pub fn add_runners(&mut self, phase: PhaseId, runner: fn(RunContext)) {
        self.runners.insert(phase, runner);
    }

    pub fn runner(&self, phase: PhaseId) -> fn(RunContext) {
        self.runners
            .get(&phase)
            .copied()
            .unwrap_or(|ctx: RunContext| ctx.run())
    }

    pub fn get_phases(&self, phase: PhaseId) -> Vec<&SystemGraph> {
        self.phases
            .values()
            .filter_map(|phases| phases.0.get(&phase).map(|systems| systems))
            .collect()
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    pub(crate) fn add_configs(&mut self) {
        if !self.configs.is_empty() {
            let configs = std::mem::take(&mut self.configs);
            let phases = self.phases.entry("default").or_insert(Phases::new());
            for (phase, configs) in configs {
                phases.add_systems(phase, configs);
            }
        }
    }

    pub fn run(&self, phase: impl Phase, world: WorldCell) {
        if phase.id() == self.schedule.id() {
            self.schedule.run(&world, self, self.mode);
        } else {
            self.schedule.run_child(phase.id(), &world, self, self.mode);
        }
    }
}
