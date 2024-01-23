use rouge_ecs::storage::sparse::SparseMap;

use crate::game::Game;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PluginId(TypeId);

impl PluginId {
    pub fn new<T: Plugin>() -> Self {
        Self(TypeId::of::<T>())
    }
}

impl From<TypeId> for PluginId {
    fn from(id: TypeId) -> Self {
        Self(id)
    }
}

pub trait Plugin: 'static {
    fn plugins(&self, _: &mut Plugins) {}
    fn start(&mut self, _: &mut Game) {}
    fn run(&mut self, _: &mut Game) {}
    fn finish(&mut self, _: &mut Game) {}
    fn dependencies(&self) -> Vec<PluginId> {
        vec![]
    }
}

pub struct Plugins {
    plugins: SparseMap<PluginId, Box<dyn Plugin>>,
}

impl Default for Plugins {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugins {
    pub fn new() -> Self {
        Self {
            plugins: SparseMap::new(),
        }
    }

    pub fn register<T: Plugin>(&mut self, plugin: T) {
        self.plugins.insert(PluginId::new::<T>(), Box::new(plugin));
    }

    pub fn extend(&mut self, plugins: &mut Plugins) {
        self.plugins.append(&mut plugins.plugins);
    }

    pub(crate) fn plugins(&mut self, plugins: &mut Plugins) {
        for plugin in self.plugins.values_mut() {
            plugin.plugins(plugins);
        }
    }

    pub(crate) fn start(&mut self, game: &mut Game) {
        for plugin in self.plugins.values_mut() {
            plugin.start(game);
        }
    }

    pub(crate) fn run(&mut self, game: &mut Game) {
        for plugin in self.plugins.values_mut() {
            plugin.run(game);
        }
    }

    pub(crate) fn finish(&mut self, game: &mut Game) {
        for plugin in self.plugins.values_mut() {
            plugin.finish(game);
        }
    }

    pub(crate) fn sort(&mut self) {
        let mut plugin_dependencies = HashMap::new();

        for (id, plugin) in self.plugins.iter() {
            let dependencies = plugin.dependencies();

            plugin_dependencies.insert(*id, HashSet::new());

            for dependency in dependencies {
                plugin_dependencies
                    .entry(dependency)
                    .or_insert_with(HashSet::new)
                    .insert(*id);
            }
        }

        let mut sorted = Vec::new();
        while !plugin_dependencies.is_empty() {
            let ready = plugin_dependencies
                .keys()
                .filter_map(|id| {
                    plugin_dependencies
                        .iter()
                        .all(|(_, dependencies)| !dependencies.contains(id))
                        .then_some(*id)
                })
                .collect::<Vec<_>>();

            for id in ready {
                sorted.insert(0, id);
                plugin_dependencies.remove(&id);
            }
        }

        self.plugins.sort_keys(|a, b| {
            let a_first = sorted
                .iter()
                .position(|id| id == a)
                .unwrap_or_else(|| panic!("Plugin {:?} not found in sorted list", a));
            let b_first = sorted
                .iter()
                .position(|id| id == b)
                .unwrap_or_else(|| panic!("Plugin {:?} not found in sorted list", b));

            a_first.cmp(&b_first)
        })
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}
