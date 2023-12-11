use self::plugin::Plugin;
use crate::ecs::{Component, ResetInterval, Resource, State, World};

pub mod plugin;

pub type Runner = Box<dyn Fn(Game)>;

pub struct Game {
    runner: Option<Runner>,
    world: World,
    plugins: Vec<Box<dyn Plugin>>,
}

impl Game {
    pub fn new() -> Game {
        Game {
            runner: None,
            world: World::new(),
            plugins: Vec::new(),
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

    pub fn add_state<T: State>(&mut self, state: T, interval: ResetInterval) -> &mut Self {
        self.world.insert_state(state, interval);

        self
    }

    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        self.plugins.push(Box::new(plugin));

        self
    }

    pub fn with_runner<T: Fn(Game) + 'static>(&mut self, runner: T) -> &mut Self {
        self.runner = Some(Box::new(runner));

        self
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn run(mut self) {
        self.run_plugins();
        (self.runner.take().unwrap())(self);
    }

    pub fn update(&mut self) -> Option<()> {
        Some(())
    }
}

impl Game {
    fn run_plugins(&mut self) {
        let mut plugins = vec![];
        while let Some(plugin) = self.plugins.pop() {
            plugins.append(&mut Self::get_recursive_plugins(plugin))
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

    fn get_recursive_plugins(plugin: Box<dyn Plugin>) -> Vec<Box<dyn Plugin>> {
        let dependencies = plugin.dependencies();
        let mut plugins = vec![plugin];
        for plugin in dependencies {
            plugins.append(&mut Self::get_recursive_plugins(plugin));
        }

        plugins
    }
}
