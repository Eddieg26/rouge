use self::{
    plugin::Plugin,
    scene::SceneManager,
    schedule::{Schedule, SchedulePlan},
    states::{
        EndFrame, EndScene, GameContext, GamePhase, InitGame, Render, Shutdown, StartScene, Update,
    },
};
use crate::{
    asset::{self, Asset, AssetLoader, AssetManager, Assets},
    ecs::{world::Event, Component, ResetInterval, Resource, State, System, World},
};
use std::collections::{HashMap, HashSet};

pub mod plugin;
pub mod scene;
pub mod schedule;
pub mod states;

pub type Runner = Box<dyn Fn(Game)>;

pub struct Game {
    runner: Runner,
    world: World,
    plugins: Vec<Box<dyn Plugin>>,
    global_plan: SchedulePlan,
    plan: SchedulePlan,
    sub_apps: HashMap<SubAppName, SubApp>,
    initialized: bool,
}

impl Game {
    pub fn new() -> Game {
        let mut world = World::empty();
        world.insert_resource(SceneManager::new());
        world.insert_resource(AssetManager::new("assets"));

        let load_schedule = Schedule::new("Load Assets")
            .add_system(asset::load_assets)
            .build();
        let mut global_plan = SchedulePlan::new();
        global_plan.add_schedule(GamePhase::Init, load_schedule);

        Game {
            world,
            runner: Box::new(default_runner),
            plugins: Vec::new(),
            global_plan: SchedulePlan::new(),
            plan: SchedulePlan::new(),
            sub_apps: HashMap::new(),
            initialized: false,
        }
    }

    pub fn register<T: Component>(&mut self) -> &mut Self {
        self.world.register::<T>();

        self
    }

    pub fn add_scene<T: scene::Scene>(&mut self, scene: T) -> &mut Self {
        self.world.resource_mut::<SceneManager>().add_scene(scene);

        self
    }

    pub fn add_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.world.insert_resource(resource);

        self
    }

    pub fn add_schedule(&mut self, schedule: Schedule) -> &mut Self {
        self.global_plan.add_schedule(GamePhase::Update, schedule);

        self
    }

    pub fn add_phase_schedule(&mut self, phase: GamePhase, schedule: Schedule) -> &mut Self {
        self.global_plan.add_schedule(phase, schedule);

        self
    }

    pub fn add_system<T: System>(&mut self, system: T) -> &mut Self {
        self.global_plan.add_system(GamePhase::Update, system);

        self
    }

    pub fn add_phase_system<T: System>(&mut self, phase: GamePhase, system: T) -> &mut Self {
        self.global_plan.add_system(phase, system);

        self
    }

    pub fn add_state<T: State>(&mut self, state: T, interval: ResetInterval) -> &mut Self {
        self.world.insert_state(state, interval);

        self
    }

    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        self.plugins.push(Box::new(plugin));

        self
    }

    pub fn add_sub_app(&mut self, name: impl Into<SubAppName>, sub_app: SubApp) -> &mut Self {
        self.sub_apps.insert(name.into(), sub_app);

        self
    }

    pub fn add_asset<A: Asset>(&mut self) -> &mut Self {
        self.world.insert_resource(Assets::<A>::new());

        self
    }

    pub fn add_loader<L: AssetLoader>(&mut self) -> &mut Self {
        self.world
            .resource_mut::<AssetManager>()
            .register_loader::<L>();

        self
    }

    pub fn add_asset_path(&mut self, path: &str) -> &mut Self {
        self.world.resource_mut::<AssetManager>().add_path(path);

        self
    }

    pub fn observe<T: Event>(
        &mut self,
        observer: impl Fn(&[T::Data], &World) + 'static,
    ) -> &mut Self {
        self.world.observe::<T>(observer);

        self
    }

    pub fn with_runner<T: Fn(Game) + 'static>(&mut self, runner: T) -> &mut Self {
        self.runner = Box::new(runner);

        self
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn run(&mut self) {
        self.run_plugins();

        let mut app = std::mem::replace(self, Self::new());
        let runner = std::mem::replace(&mut app.runner, Box::new(default_runner));

        (runner)(app);
    }

    pub fn update(&mut self) -> Option<()> {
        if !self.initialized {
            self.initialized = true;

            let mut ctx: GameContext<'_> = GameContext::from_game(self);
            InitGame::execute(&mut ctx);
        }

        if self.world.resource::<SceneManager>().current().is_none() {
            self.world.resource_mut::<SceneManager>().start();
            let mut ctx: GameContext<'_> = GameContext::from_game(self);
            StartScene::execute(&mut ctx);
        } else if self.world.resource::<SceneManager>().next().is_some() {
            let mut ctx: GameContext<'_> = GameContext::from_game(self);
            EndScene::execute(&mut ctx);
            StartScene::execute(&mut ctx);
        }

        let mut ctx: GameContext<'_> = GameContext::from_game(self);
        Update::execute(&mut ctx);
        Render::execute(&mut ctx);
        EndFrame::execute(&mut ctx);

        for sub_app in self.sub_apps.values_mut() {
            sub_app.extract(&mut self.world);
            sub_app.update();
        }

        if self.world.resource::<SceneManager>().quitting() {
            Shutdown::execute(&mut GameContext::from_game(self));
            return None;
        } else {
            Some(())
        }
    }
}

impl Game {
    fn run_plugins(&mut self) {
        let mut plugins = vec![];
        let mut inserted = HashSet::new();
        while let Some(plugin) = self.plugins.pop() {
            plugins.append(&mut Self::get_recursive_plugins(plugin, &mut inserted))
        }
        plugins.reverse();

        for plugin in &plugins {
            plugin.start(self);
        }

        for plugin in &plugins {
            plugin.run(self);
        }

        for plugin in &plugins {
            plugin.finish(self);
        }
    }

    fn get_recursive_plugins(
        plugin: Box<dyn Plugin>,
        inserted: &mut HashSet<String>,
    ) -> Vec<Box<dyn Plugin>> {
        let dependencies = plugin.dependencies();
        let mut plugins = vec![plugin];
        for plugin in dependencies {
            if inserted.contains(plugin.name()) {
                continue;
            }

            inserted.insert(plugin.name().to_string());
            plugins.append(&mut Self::get_recursive_plugins(plugin, inserted));
        }

        plugins
    }
}

fn default_runner(mut game: Game) {
    game.update();
}

pub struct SubApp {
    world: World,
    extract: Box<dyn Fn(&mut World, &mut World)>,
    plan: SchedulePlan,
    dummy: SchedulePlan,
}

impl Default for SubApp {
    fn default() -> Self {
        Self::new(|_, _| {})
    }
}

impl SubApp {
    pub fn new(extract: impl Fn(&mut World, &mut World) + 'static) -> SubApp {
        SubApp {
            world: World::empty(),
            extract: Box::new(extract),
            plan: SchedulePlan::new(),
            dummy: SchedulePlan::new(),
        }
    }

    pub fn register<T: Component>(&mut self) -> &mut Self {
        self.world.register::<T>();

        self
    }

    pub fn add_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.world.insert_resource(resource);

        self
    }

    pub fn add_schedule(&mut self, phase: GamePhase, schedule: Schedule) -> &mut Self {
        self.plan.add_schedule(phase, schedule);

        self
    }

    pub fn add_system<T: System>(&mut self, system: T) -> &mut Self {
        self.plan.add_system(GamePhase::Update, system);

        self
    }

    pub fn add_state<T: State>(&mut self, state: T, interval: ResetInterval) -> &mut Self {
        self.world.insert_state(state, interval);

        self
    }

    pub fn finish(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn observe<T: Event>(
        &mut self,
        observer: impl Fn(&[T::Data], &World) + 'static,
    ) -> &mut Self {
        self.world.observe::<T>(observer);

        self
    }

    pub fn update(&mut self) {
        let mut ctx = GameContext::new(&mut self.world, &mut self.plan, &mut self.dummy);
        Update::execute(&mut ctx);
        Render::execute(&mut ctx);
        EndFrame::execute(&mut ctx);
    }

    pub(super) fn extract(&mut self, world: &mut World) {
        (self.extract)(world, &mut self.world);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubAppName(String);

impl SubAppName {
    pub fn new(name: &str) -> SubAppName {
        SubAppName(name.to_string())
    }
}

impl std::ops::Deref for SubAppName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a str> for SubAppName {
    fn from(name: &'a str) -> Self {
        SubAppName::new(name)
    }
}

impl From<String> for SubAppName {
    fn from(name: String) -> Self {
        SubAppName(name)
    }
}
