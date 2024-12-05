use crate::phases::{Extract, Update};
use ecs::{
    core::{component::Component, resource::Resource, IndexMap, Type},
    event::Event,
    system::{schedule::Phase, ArgItem, IntoSystemConfigs, SystemArg, WorldAccess},
    task::TaskPool,
    world::{
        action::{WorldActionFn, WorldActions},
        cell::WorldCell,
        World,
    },
};
use std::sync::{Arc, Mutex};

pub trait AppTag: 'static + Send {
    const NAME: &'static str;
}

pub struct SubApp {
    world: World,
}

impl SubApp {
    pub fn new() -> Self {
        Self {
            world: World::sub(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.world.resource_mut::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.world.non_send_resource::<R>()
    }

    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.world.non_send_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.world.try_resource_mut::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_non_send_resource::<R>()
    }

    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.try_non_send_resource_mut::<R>()
    }

    pub fn has_resource<R: Resource + Send>(&self) -> bool {
        self.world.has_resource::<R>()
    }

    pub fn has_non_send_resource<R: Resource>(&self) -> bool {
        self.world.has_non_send_resource::<R>()
    }

    pub fn actions(&self) -> &WorldActions {
        self.world.actions()
    }

    pub fn tasks(&self) -> &TaskPool {
        self.world.tasks()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.world.register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn register_resource<R: Resource + Send>(&mut self) -> &mut Self {
        self.world.register_resource::<R>();
        self
    }

    pub fn register_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.world.register_non_send_resource::<R>();
        self
    }

    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.world.add_resource(resource);
        self
    }

    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.add_non_send_resource::<R>(resource);
        self
    }

    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.world.remove_resource::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_non_send_resource::<R>()
    }

    pub fn scoped_resource<R: Resource + Send>(
        &mut self,
        scope: impl FnOnce(&mut World, &mut R),
    ) -> &mut Self {
        self.world.scoped_resource::<R>(scope);
        self
    }

    pub fn scoped_non_send_resource<R: Resource>(
        &mut self,
        scope: impl FnOnce(&mut World, &mut R),
    ) -> &mut Self {
        self.world.scoped_non_send_resource::<R>(scope);
        self
    }

    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.world.add_systems::<M>(phase, systems);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.world.observe::<E, M>(observers);
        self
    }

    pub fn add_extract_phase<P: Phase>(&mut self) -> &mut Self {
        self.world.add_sub_phase::<Extract, P>();
        self
    }

    pub fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.world.add_sub_phase::<Update, P>();
        self
    }

    pub fn add_phase_before<P: Phase, Before: Phase>(&mut self) -> &mut Self {
        self.world.add_phase_before::<P, Before>();
        self
    }

    pub fn add_phase_after<P: Phase, After: Phase>(&mut self) -> &mut Self {
        self.world.add_phase_after::<P, After>();
        self
    }

    pub(crate) fn extract(&mut self, main: MainWorld) {
        self.world.add_resource(main);
        self.world.flush(None);
        self.world.run(Extract);
        self.world.remove_resource::<MainWorld>();
    }

    pub(crate) fn run(&mut self) {
        self.world.run(Update);
    }
}

pub struct MainApp {
    world: World,
}

impl MainApp {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn run(&mut self, phase: impl Phase) {
        self.world.run(phase);
    }
}

impl Default for MainApp {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AppBuilders {
    main: MainApp,
    apps: IndexMap<Type, SubApp>,
}

impl AppBuilders {
    pub fn new() -> Self {
        Self {
            main: MainApp::new(),
            apps: IndexMap::new(),
        }
    }

    pub fn main_app(&self) -> &MainApp {
        &self.main
    }

    pub fn main_app_mut(&mut self) -> &mut MainApp {
        &mut self.main
    }

    pub fn main_world(&self) -> &World {
        &self.main.world
    }

    pub fn main_world_mut(&mut self) -> &mut World {
        &mut self.main.world
    }

    pub fn sub<A: AppTag>(&self) -> Option<&SubApp> {
        self.apps.get(&Type::of::<A>())
    }

    pub fn sub_mut<A: AppTag>(&mut self) -> Option<&mut SubApp> {
        self.apps.get_mut(&Type::of::<A>())
    }

    pub fn sub_dyn(&self, tag: Type) -> Option<&SubApp> {
        self.apps.get(&tag)
    }

    pub fn sub_dyn_mut(&mut self, tag: Type) -> Option<&mut SubApp> {
        self.apps.get_mut(&tag)
    }

    pub fn add<A: AppTag>(&mut self) -> &mut SubApp {
        let ty = Type::of::<A>();
        if !self.apps.contains_key(&ty) {
            let mut app = SubApp::new();
            app.register_resource::<MainWorld>();
            app.world.add_phase::<Extract>();
            app.world.add_phase::<Update>();

            self.apps.insert(ty, app);
        }

        self.apps.get_mut(&ty).unwrap()
    }

    pub fn remove<A: AppTag>(&mut self) -> Option<SubApp> {
        self.apps.shift_remove(&Type::of::<A>())
    }

    pub fn insert(&mut self, tag: Type, app: SubApp) {
        self.apps.insert(tag, app);
    }

    pub fn into_apps(&mut self) -> Apps {
        Apps {
            main: std::mem::take(&mut self.main),
            apps: self
                .apps
                .drain(..)
                .map(|(k, v)| (k, Arc::new(Mutex::new(v))))
                .collect(),
            tasks: TaskPool::default(),
        }
    }
}

pub struct Apps {
    main: MainApp,
    apps: IndexMap<Type, Arc<Mutex<SubApp>>>,
    tasks: TaskPool,
}

impl Apps {
    pub fn new() -> Self {
        Self {
            main: MainApp::new(),
            apps: IndexMap::new(),
            tasks: TaskPool::default(),
        }
    }

    pub fn main_app(&self) -> &MainApp {
        &self.main
    }

    pub fn main_app_mut(&mut self) -> &mut MainApp {
        &mut self.main
    }

    pub fn main_world(&self) -> &World {
        &self.main.world
    }

    pub fn main_world_mut(&mut self) -> &mut World {
        &mut self.main.world
    }

    pub fn run(&mut self) {
        let main = MainWorld::new(self.main.world_mut());
        for app in self.apps.values_mut() {
            let mut app_lock = app.lock().unwrap();
            app_lock.extract(main);

            let app = app.clone();
            self.tasks.spawn(move || {
                let mut app_lock = app.lock().unwrap();
                app_lock.run();
            });
        }
    }
}

impl Default for Apps {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MainWorld(*mut World);
impl MainWorld {
    pub(crate) fn new(world: &mut World) -> Self {
        Self(world as *mut World)
    }

    pub fn inner(&self) -> &World {
        unsafe { &*self.0 }
    }

    pub fn inner_mut(&mut self) -> &mut World {
        unsafe { &mut *self.0 }
    }
}

impl std::ops::Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl std::ops::DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}

impl Resource for MainWorld {}
unsafe impl Send for MainWorld {}
unsafe impl Sync for MainWorld {}

#[derive(Clone)]
pub struct MainActions(WorldActions);

impl MainActions {
    pub fn new(actions: WorldActions) -> Self {
        Self(actions)
    }

    pub fn add(&self, action: impl Into<WorldActionFn>) {
        self.0.add(action);
    }

    pub fn extend(&self, actions: Vec<impl Into<WorldActionFn>>) {
        self.0.extend(actions);
    }
}

impl Resource for MainActions {}

pub struct SubActions<A: AppTag>(WorldActions, std::marker::PhantomData<A>);
impl<A: AppTag> SubActions<A> {
    pub fn new(actions: WorldActions) -> Self {
        Self(actions, std::marker::PhantomData)
    }
}

impl<A: AppTag> Resource for SubActions<A> {}

impl<A: AppTag> Clone for SubActions<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), std::marker::PhantomData)
    }
}

impl<A: AppTag> std::ops::Deref for SubActions<A> {
    type Target = WorldActions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: AppTag> std::ops::DerefMut for SubActions<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Main<'w, S: SystemArg>(ArgItem<'w, S>);

impl<'w, S: SystemArg> std::ops::Deref for Main<'w, S> {
    type Target = ArgItem<'w, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, S: SystemArg> std::ops::DerefMut for Main<'w, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, P: SystemArg> Main<'w, P> {
    pub fn into_inner(self) -> ArgItem<'w, P> {
        self.0
    }
}

impl<S: SystemArg + 'static> SystemArg for Main<'_, S> {
    type Item<'world> = Main<'world, S>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        let world = world.resource::<MainWorld>();
        Main(S::get(world.inner().into()))
    }

    fn access() -> Vec<WorldAccess> {
        S::access()
    }
}

#[cfg(test)]

mod test {
    use crate::{
        app::AppTag,
        game::Game,
        phases::{Extract, Update},
    };

    #[test]
    fn sub_app() {
        struct TestApp;
        impl AppTag for TestApp {
            const NAME: &'static str = "TestApp";
        }
        let mut game = Game::new();
        game.add_systems(Update, || {});
        {
            let app = game.add_sub_app::<TestApp>();
            app.add_systems(Extract, || {});
            app.add_systems(Update, || {});
        }

        game.set_runner(|mut game: Game| {
            game.startup();

            let instant = std::time::Instant::now();

            loop {
                game.update();
                if instant.elapsed().as_secs() >= 1 {
                    break;
                }
            }

            game.shutdown();
        });

        game.run();
    }
}
