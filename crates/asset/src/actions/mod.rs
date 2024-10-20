use crate::database::config::AssetConfig;
use ecs::{
    event::{Event, Events},
    world::action::{WorldAction, WorldActions},
};
use std::collections::VecDeque;

pub mod import;

pub trait AssetAction: Send + Sync + 'static {
    fn execute(&mut self, config: &AssetConfig, actions: &WorldActions);
}

pub struct StartAssetAction {
    action: Box<dyn AssetAction>,
}

impl StartAssetAction {
    pub fn new(action: impl AssetAction) -> Self {
        Self {
            action: Box::new(action),
        }
    }

    pub fn take(self) -> Box<dyn AssetAction> {
        self.action
    }
}

impl Event for StartAssetAction {}

impl WorldAction for StartAssetAction {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetAction>>();
        Some(events.add(self))
    }
}

pub struct AssetActions {
    actions: VecDeque<Box<dyn AssetAction>>,
    running: bool,
}

impl AssetActions {
    pub fn new() -> Self {
        Self {
            actions: VecDeque::new(),
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub(crate) fn start(&mut self) {
        self.running = true;
    }

    pub(crate) fn stop(&mut self) {
        self.running = false;
    }

    pub fn push(&mut self, action: impl AssetAction) {
        self.actions.push_back(Box::new(action));
    }

    pub fn push_front(&mut self, action: impl AssetAction) {
        self.actions.push_front(Box::new(action));
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetAction>> {
        self.actions.pop_front()
    }

    pub fn extend(&mut self, actions: impl IntoIterator<Item = impl Into<Box<dyn AssetAction>>>) {
        self.actions.extend(actions.into_iter().map(|a| a.into()));
    }
}
