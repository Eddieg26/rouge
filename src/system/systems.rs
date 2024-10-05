use super::{
    schedule::{
        Phase, PhaseId, PhaseRunner, PhaseRunners, PhaseSystemConfigs, PhaseSystemGraphs,
        RunContext, Schedule, SystemGraph,
    },
    IntoSystemConfigs,
};
use crate::{core::Type, task::ScopedTaskPool, world::cell::WorldCell};
use indexmap::IndexMap;
use std::{
    num::NonZero,
    sync::{Arc, Mutex, MutexGuard},
};

pub trait SystemGroup: 'static {}
pub struct Global;
impl SystemGroup for Global {}

pub struct SystemConfigs {
    configs: IndexMap<Type, PhaseSystemConfigs>,
    meta: SystemMeta,
}

impl SystemConfigs {
    pub fn new(mode: RunMode) -> Self {
        let mut configs = IndexMap::new();
        configs.insert(Type::of::<Global>(), PhaseSystemConfigs::new());

        Self {
            configs,
            meta: SystemMeta::new(mode),
        }
    }

    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    pub fn mode(&self) -> RunMode {
        self.meta.mode()
    }

    pub fn len(&self) -> usize {
        self.configs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    pub fn add_systems<M>(
        &mut self,
        ty: Type,
        phase: impl Phase,
        configs: impl IntoSystemConfigs<M>,
    ) {
        self.configs
            .entry(ty)
            .or_default()
            .add_systems(phase, configs);
    }

    pub fn add_phase_configs<G: SystemGroup>(&mut self, configs: PhaseSystemConfigs) {
        self.configs.insert(Type::of::<G>(), configs);
    }

    pub fn build_graphs(&mut self) -> SystemGraphs {
        let graphs = self
            .configs
            .drain(..)
            .map(|(ty, c)| (ty, c.into_graphs(self.meta.mode())));
        SystemGraphs::with_graphs(graphs.collect())
    }
}

pub struct SystemGraphs {
    global: Type,
    graphs: IndexMap<Type, PhaseSystemGraphs>,
}

impl SystemGraphs {
    pub fn new() -> Self {
        let global = Type::of::<Global>();
        let mut graphs = IndexMap::new();
        graphs.insert(global, PhaseSystemGraphs::new());

        Self { global, graphs }
    }

    pub fn global(&self) -> Type {
        self.global
    }

    pub fn with_graphs(graphs: IndexMap<Type, PhaseSystemGraphs>) -> Self {
        let global = Type::of::<Global>();

        Self { global, graphs }
    }

    pub fn add_graphs(&mut self, graphs: SystemGraphs) {
        self.graphs.extend(graphs.graphs);
    }

    pub fn remove_graphs(&mut self, ty: Type) {
        if ty != self.global {
            self.graphs.shift_remove(&ty);
        }
    }

    pub fn get(&self, id: PhaseId) -> Vec<&SystemGraph> {
        self.graphs
            .values()
            .filter_map(|graphs| graphs.get(id))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunMode {
    Sequential,
    Parallel,
}

impl RunMode {
    pub fn runner(&self) -> Arc<dyn SystemRunner> {
        match self {
            RunMode::Sequential => Arc::new(SequentialRunner),
            RunMode::Parallel => Arc::new(ParallelRunner),
        }
    }

    pub fn max_threads() -> usize {
        std::thread::available_parallelism()
            .unwrap_or(NonZero::<usize>::new(1).unwrap())
            .into()
    }
}

pub trait SystemRunner: Send + Sync + 'static {
    fn run(&self, world: &WorldCell, systems: &[&SystemGraph]);
}

pub struct SequentialRunner;
impl SystemRunner for SequentialRunner {
    fn run(&self, world: &WorldCell, systems: &[&SystemGraph]) {
        for graph in systems {
            for system in graph.systems() {
                system.run(world);
            }
        }
    }
}

pub struct ParallelRunner;
impl SystemRunner for ParallelRunner {
    fn run(&self, world: &WorldCell, systems: &[&SystemGraph]) {
        for graph in systems {
            for group in graph.order() {
                let mut pool = ScopedTaskPool::new(RunMode::max_threads() as usize);
                for index in group {
                    pool.spawn(move || graph.systems()[*index].run(world));
                }

                pool.run();
            }
        }
    }
}

#[derive(Clone)]
pub struct SystemMeta {
    mode: RunMode,
    runner: Arc<dyn SystemRunner>,
    phase_runners: Arc<Mutex<PhaseRunners>>,
}

impl SystemMeta {
    pub fn new(mode: RunMode) -> Self {
        Self {
            mode,
            runner: mode.runner(),
            phase_runners: Arc::default(),
        }
    }

    pub fn mode(&self) -> RunMode {
        self.mode
    }

    pub fn runner(&self) -> &Arc<dyn SystemRunner> {
        &self.runner
    }

    pub fn phase_runners(&self) -> Arc<Mutex<PhaseRunners>> {
        self.phase_runners.clone()
    }

    pub fn add_phase_runner(&mut self, phase: impl Phase, runner: impl PhaseRunner) {
        self.phase_runners
            .lock()
            .unwrap()
            .add_runner(phase.id(), runner);
    }
}

pub struct PhaseRunnersRef<'a> {
    runners: MutexGuard<'a, PhaseRunners>,
}

impl<'a> PhaseRunnersRef<'a> {
    pub fn new(runners: MutexGuard<'a, PhaseRunners>) -> Self {
        Self { runners }
    }
}
impl<'a> std::ops::Deref for PhaseRunnersRef<'a> {
    type Target = PhaseRunners;

    fn deref(&self) -> &Self::Target {
        &self.runners
    }
}
impl<'a> std::ops::DerefMut for PhaseRunnersRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runners
    }
}

pub struct Root;
impl Phase for Root {}

pub struct Systems {
    graphs: SystemGraphs,
    schedule: Schedule,
}

impl Systems {
    #[inline]
    pub fn new() -> Self {
        Self {
            graphs: SystemGraphs::new(),
            schedule: Root::schedule(),
        }
    }

    #[inline]
    pub fn graphs(&self) -> &SystemGraphs {
        &self.graphs
    }

    #[inline]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    #[inline]
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    #[inline]
    pub fn add_graphs(&mut self, graphs: SystemGraphs) {
        self.graphs.add_graphs(graphs);
    }

    #[inline]
    pub fn run(&self, phase: impl Phase, world: WorldCell) {
        let meta = world.get().configs().meta();
        let runners = meta.phase_runners();
        let mut runners = PhaseRunnersRef::new(runners.lock().unwrap());
        if phase.id() == self.schedule.id() {
            self.schedule.run(&world, self, &meta, &mut runners);
        } else {
            self.schedule
                .run_child(phase.id(), &world, self, &meta, &mut runners);
        }
    }
}

impl Schedule {
    pub fn run(
        &self,
        world: &WorldCell,
        systems: &Systems,
        meta: &SystemMeta,
        runners: &mut PhaseRunnersRef,
    ) {
        let graphs = systems.graphs().get(self.id());
        if !graphs.is_empty() {
            let ctx = RunContext::new(world, &graphs, meta.runner().as_ref());
            let runner = runners.get_mut(&self.id());
            runner.run(ctx);
        }

        world.get_mut().flush(Some(self.id()));

        for child in self.children() {
            child.run(world, systems, meta, runners);
        }
    }

    pub fn run_child(
        &self,
        child: PhaseId,
        world: &WorldCell,
        systems: &Systems,
        meta: &SystemMeta,
        runners: &mut PhaseRunnersRef,
    ) {
        if let Some(child) = self.child(child, true) {
            child.run(world, systems, meta, runners);
        }
    }
}
