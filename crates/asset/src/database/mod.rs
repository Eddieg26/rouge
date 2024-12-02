use crate::io::{
    cache::{LoadPath, SharedLibrary},
    source::AssetPath,
    AssetIoError,
};
use async_std::sync::{Mutex, RwLock};
use config::AssetConfig;
use ecs::{core::resource::Resource, event::Event, task::TaskPool, world::action::WorldActions};
use futures::executor::block_on;
use state::{LoadState, SharedStates};
use std::{collections::VecDeque, sync::Arc};
use update::{AssetImporter, AssetLoader, AssetRefresher, RefreshMode};

pub mod config;
pub mod events;
pub mod state;
pub mod update;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseState {
    Idle,
    Updating,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseEvent {
    Import(Vec<AssetPath>),
    Refresh(RefreshMode),
    Load(Vec<LoadPath>),
}

pub enum DatabaseInitError {
    Io(AssetIoError),
}

impl Event for DatabaseInitError {}

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    tasks: TaskPool,
    actions: WorldActions,
    library: SharedLibrary,
    states: SharedStates,
    state: Arc<Mutex<DatabaseState>>,
    events: Arc<Mutex<VecDeque<DatabaseEvent>>>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig, tasks: TaskPool, actions: WorldActions) -> Self {
        Self {
            tasks,
            actions,
            config: Arc::new(config),
            library: SharedLibrary::default(),
            states: SharedStates::default(),
            state: Arc::new(Mutex::new(DatabaseState::Idle)),
            events: Arc::default(),
        }
    }

    pub(crate) async fn init(&mut self) -> Result<(), DatabaseInitError> {
        let library = self
            .config
            .cache()
            .init()
            .await
            .map_err(DatabaseInitError::Io)?;
        self.library = Arc::new(RwLock::new(library));
        Ok(())
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn state(&self) -> DatabaseState {
        *self.state.lock_arc_blocking()
    }

    pub fn library(&self) -> &SharedLibrary {
        &self.library
    }

    pub fn states(&self) -> &SharedStates {
        &self.states
    }

    pub fn asset_load_state(&self, path: LoadPath) -> LoadState {
        let id = match path {
            LoadPath::Id(id) => id,
            LoadPath::Path(path) => {
                let library = self.library.read_arc_blocking();
                match library.get_id(&path) {
                    Some(id) => id,
                    None => return LoadState::Unloaded,
                }
            }
        };

        self.states.read_blocking().get_load_state(id)
    }

    pub fn refresh(&self, mode: RefreshMode) {
        self.events
            .lock_arc_blocking()
            .push_front(DatabaseEvent::Refresh(mode));

        self.update();
    }

    pub fn import(&self, paths: impl IntoIterator<Item = impl Into<AssetPath>>) {
        let paths = paths.into_iter().map(Into::into).collect::<Vec<_>>();
        if !paths.is_empty() {
            self.events
                .lock_arc_blocking()
                .push_back(DatabaseEvent::Import(paths));
            self.update();
        }
    }

    pub fn load(&self, paths: impl IntoIterator<Item = impl Into<LoadPath>>) {
        let paths = paths.into_iter().map(Into::into).collect::<Vec<_>>();
        if !paths.is_empty() {
            self.events
                .lock_arc_blocking()
                .push_back(DatabaseEvent::Load(paths));
            self.update();
        }
    }

    fn update(&self) {
        let mut state_lock = self.state.lock_arc_blocking();
        if *state_lock == DatabaseState::Idle {
            *state_lock = DatabaseState::Updating;
            let database = self.clone();
            self.tasks.spawn(move || {
                block_on(database.run());
                let mut state = database.state.lock_arc_blocking();
                *state = DatabaseState::Idle;
            });
        }
    }

    async fn run(&self) {
        let mut events = self.events();

        while !events.is_empty() {
            for event in events {
                match event {
                    DatabaseEvent::Refresh(mode) => AssetRefresher.refresh(mode, self).await,
                    DatabaseEvent::Import(paths) => AssetImporter.import(paths, self).await,
                    DatabaseEvent::Load(paths) => AssetLoader.load(paths, self).await,
                }
            }

            events = self.events();
        }
    }

    fn events(&self) -> Vec<DatabaseEvent> {
        self.events.lock_arc_blocking().drain(..).collect()
    }
}

impl Resource for AssetDatabase {}
