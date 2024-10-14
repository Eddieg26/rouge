use super::AssetDatabase;
use ecs::{
    event::{Event, Events},
    task::TaskPool,
    world::action::WorldActions,
};
use std::collections::VecDeque;

pub mod import;
pub mod load;

pub trait AssetEvent: Send + Sync + 'static {
    fn execute(&mut self, database: &AssetDatabase, actions: &WorldActions);
}

impl<A: AssetEvent> From<A> for Box<dyn AssetEvent> {
    fn from(event: A) -> Self {
        Box::new(event)
    }
}

pub struct AssetEvents {
    events: VecDeque<Box<dyn AssetEvent>>,
    running: bool,
}

impl AssetEvents {
    pub(crate) fn new() -> Self {
        Self {
            events: VecDeque::new(),
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    fn start(&mut self) {
        self.running = true;
    }

    fn stop(&mut self) {
        self.running = false;
    }

    pub fn push(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_back(event.into());
    }

    pub fn push_front(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_front(event.into());
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetEvent>> {
        self.events.pop_front()
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = impl Into<Box<dyn AssetEvent>>>) {
        self.events.extend(events.into_iter().map(Into::into));
    }

    pub(crate) fn execute(database: AssetDatabase, actions: WorldActions) {
        while let Some(mut event) = database.events().pop() {
            event.execute(&database, &actions);
        }

        database.events.lock().unwrap().stop();
    }

    pub(crate) fn observer(
        asset_events: &mut Events<StartAssetEvent>,
        actions: &WorldActions,
        database: &AssetDatabase,
        tasks: &TaskPool,
    ) {
        let mut events = database.events();
        if !events.is_running() {
            events.start();
            events.extend(asset_events.drain().map(|e| e.event));
            let database = database.clone();
            let actions = actions.clone();

            tasks.spawn(move || {
                Self::execute(database, actions);
            });
        } else {
            events.extend(asset_events.drain());
        }
    }
}

pub struct StartAssetEvent {
    pub event: Box<dyn AssetEvent>,
}

impl StartAssetEvent {
    pub fn new(event: impl AssetEvent) -> Self {
        Self {
            event: Box::new(event),
        }
    }
}

impl<A: AssetEvent> From<A> for StartAssetEvent {
    fn from(event: A) -> Self {
        Self {
            event: Box::new(event),
        }
    }
}

impl Into<Box<dyn AssetEvent>> for StartAssetEvent {
    fn into(self) -> Box<dyn AssetEvent> {
        self.event
    }
}

impl Event for StartAssetEvent {}
