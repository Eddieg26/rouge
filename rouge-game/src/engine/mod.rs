use rouge_ecs::{
    core::Component,
    schedule::{Schedule, SchedulePhase},
    storage::sparse::SparseMap,
    system::{
        observer::{Action, Observers},
        IntoSystem,
    },
    world::{
        resource::{LocalResource, Resource},
        World,
    },
};
use std::any::TypeId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineId(TypeId);

impl EngineId {
    pub fn new<T: Engine>() -> Self {
        Self(TypeId::of::<T>())
    }
}

impl From<TypeId> for EngineId {
    fn from(id: TypeId) -> Self {
        Self(id)
    }
}

pub trait Engine: Send + Sync + 'static {
    fn extract(&mut self, src: &mut World, dst: &mut World);
    fn update(&mut self, world: &mut World);
}

pub struct GameEngine {
    world: World,
    engine: Box<dyn Engine>,
}

impl GameEngine {
    pub fn new(engine: impl Engine) -> Self {
        Self {
            world: World::new(),
            engine: Box::new(engine),
        }
    }

    pub fn register<C: Component>(&mut self) {
        self.world.register::<C>();
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) {
        self.world.add_resource(resource);
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) {
        self.world.add_local_resource(resource);
    }

    pub fn add_system<M>(&mut self, phase: impl SchedulePhase, system: impl IntoSystem<M>)
    where
        M: Send + Sync + 'static,
    {
        self.world.add_system(phase, system);
    }

    pub fn add_schedule(&mut self, phase: impl SchedulePhase, schedule: Schedule) {
        self.world.add_schedule(phase, schedule);
    }

    pub fn add_observers<A: Action>(&mut self, observers: Observers<A>) -> &mut Self {
        self.world.add_observers(observers);

        self
    }

    pub(crate) fn extract(&mut self, src: &mut World) {
        self.engine.extract(src, &mut self.world);
    }

    pub(crate) fn update(&mut self) {
        self.engine.update(&mut self.world);
    }
}

pub struct Engines {
    engines: SparseMap<EngineId, GameEngine>,
}

impl Engines {
    pub fn new() -> Self {
        Self {
            engines: SparseMap::new(),
        }
    }

    pub fn register<E: Engine>(&mut self, engine: E) {
        let id = EngineId::new::<E>();
        self.engines.insert(id, GameEngine::new(engine));
    }

    pub fn get<E: Engine>(&self) -> &GameEngine {
        let id = EngineId::new::<E>();
        self.engines.get(&id).expect("Engine not found")
    }

    pub fn get_mut<E: Engine>(&mut self) -> &mut GameEngine {
        let id = EngineId::new::<E>();
        self.engines.get_mut(&id).expect("Engine not found")
    }

    pub fn iter(&self) -> impl Iterator<Item = &GameEngine> {
        self.engines.iter().map(|(_, engine)| engine)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GameEngine> {
        self.engines.iter_mut().map(|(_, engine)| engine)
    }
}
