use crate::phases::{Extract, Update};
use ecs::{
    core::{resource::Resource, IndexMap, Type},
    system::schedule::Phase,
    task::TaskPool,
    world::World,
};
use std::sync::{Arc, Mutex};

pub trait AppTag: 'static {
    const NAME: &'static str;
}

pub struct SubApp {
    world: World,
}

impl SubApp {
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

    pub fn extract(&mut self, main: MainWorld) {
        self.world.add_resource(main);
        self.world.run(Extract);
        self.world.remove_resource::<MainWorld>();
    }

    pub fn run(&mut self) {
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
            app.world_mut().register_resource::<MainWorld>();
            app.world_mut().add_phase::<Extract>();
            app.world_mut().add_phase::<Update>();

            self.apps.insert(ty, app);
        }

        self.apps.get_mut(&ty).unwrap()
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

impl Resource for MainWorld {}
unsafe impl Send for MainWorld {}
unsafe impl Sync for MainWorld {}

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
