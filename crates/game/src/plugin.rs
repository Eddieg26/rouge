use crate::game::GameBuilder;
use ecs::core::{IndexMap, Type};

pub trait Plugin: 'static {
    fn name(&self) -> &'static str;
    fn start(&mut self, _game: &mut GameBuilder) {}
    fn run(&mut self, _game: &mut GameBuilder) {}
    fn finish(&mut self, _game: &mut GameBuilder) {}
    fn dependencies(&self) -> Plugins {
        Plugins::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginId(Type);
impl PluginId {
    pub fn of<P: Plugin>() -> Self {
        Self(Type::of::<P>())
    }
}

#[derive(Default)]
pub struct Plugins {
    plugins: IndexMap<PluginId, Box<dyn Plugin>>,
}

impl Plugins {
    pub fn new() -> Self {
        Self {
            plugins: IndexMap::new(),
        }
    }

    pub fn add<P: Plugin>(&mut self, plugin: P) {
        let id = PluginId::of::<P>();
        if !self.plugins.contains_key(&id) {
            self.plugins.insert(id, Box::new(plugin));
        }
    }

    pub fn flatten(&mut self) -> Plugins {
        let mut plugins = Plugins::new();
        for (id, plugin) in self.plugins.drain(..) {
            plugins.extend(plugin.dependencies().flatten());
            plugins.plugins.insert(id, plugin);
        }
        plugins
    }

    pub fn extend(&mut self, other: Plugins) {
        for (id, plugin) in other.plugins {
            self.plugins.insert(id, plugin);
        }
    }

    pub fn contains(&self, id: PluginId) -> bool {
        self.plugins.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PluginId, &Box<dyn Plugin>)> {
        self.plugins.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&PluginId, &mut Box<dyn Plugin>)> {
        self.plugins.iter_mut()
    }

    pub fn start(&mut self, game: &mut GameBuilder) {
        for plugin in self.plugins.values_mut() {
            plugin.start(game);
        }
    }

    pub fn run(&mut self, game: &mut GameBuilder) {
        for plugin in self.plugins.values_mut() {
            plugin.run(game);
        }
    }

    pub fn finish(&mut self, game: &mut GameBuilder) {
        for plugin in self.plugins.values_mut() {
            plugin.finish(game);
        }
    }
}
