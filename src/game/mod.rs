use self::plugin::Plugin;
use crate::ecs::{world::Event, Component, ResetInterval, Resource, State, World};
use std::collections::HashSet;

pub mod plugin;

pub type Runner = Box<dyn Fn(Game)>;

pub struct Game {
    runner: Runner,
    world: World,
    plugins: Vec<Box<dyn Plugin>>,
}

impl Game {
    pub fn new() -> Game {
        Game {
            runner: Box::new(default_runner),
            world: World::empty(),
            plugins: Vec::new(),
        }
    }

    fn empty() -> Game {
        Game {
            runner: Box::new(default_runner),
            world: World::empty(),
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

        let mut app = std::mem::replace(self, Self::empty());
        let runner = std::mem::replace(&mut app.runner, Box::new(default_runner));

        (runner)(app);
    }

    pub fn update(&mut self) -> Option<()> {
        Some(())
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
