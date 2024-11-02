use super::{
    state::{AssetState, AssetStates},
    AssetDatabase,
};
use crate::{
    asset::{Asset, AssetId, Assets},
    importer::LoadError,
    io::{cache::AssetLoadPath, source::AssetPath},
};
use ecs::{
    core::resource::{Res, ResMut},
    event::{Event, Events},
    world::action::{WorldAction, WorldActions},
};
use hashbrown::HashSet;

pub enum AssetEvent<A: Asset> {
    Imported(AssetId),
    Loaded(AssetId),
    Unloaded {
        id: AssetId,
        asset: Option<A>,
        state: AssetState,
    },
    DepsLoaded(AssetId),
    DepsUnloaded(AssetId),
    Failed {
        id: AssetId,
        error: LoadError,
    },
}

impl<A: Asset> Event for AssetEvent<A> {}

pub(crate) fn on_asset_event<A: Asset>(
    events: Res<Events<AssetEvent<A>>>,
    database: Res<AssetDatabase>,
    mut deps_unloaded: ResMut<Events<NotifyDepsUnloaded>>,
    actions: &WorldActions,
) {
    let mut unloaded = HashSet::new();
    for event in events.iter() {
        match event {
            AssetEvent::Unloaded { state, .. } => {
                let states = database.states.read_arc_blocking();
                on_asset_unloaded(state, &states, &mut unloaded);
                for child in state.children() {
                    let meta = match database.config().registry().get(child.ty()) {
                        Some(meta) => meta,
                        None => continue,
                    };

                    actions.add(meta.unloaded(*child));
                }
            }
            _ => {}
        }
    }

    if !unloaded.is_empty() {
        deps_unloaded.add(NotifyDepsUnloaded::new(unloaded));
    }
}

fn on_asset_unloaded(
    state: &AssetState,
    states: &AssetStates,
    deps_unloaded: &mut HashSet<AssetId>,
) {
    for dependent in state.dependents() {
        let dep_state = match states.get(dependent) {
            Some(state) => state,
            None => continue,
        };

        deps_unloaded.insert(*dependent);
        on_asset_unloaded(dep_state, states, deps_unloaded);
    }
}

pub(crate) fn on_assets_unloaded(
    mut unloaded: ResMut<Events<NotifyDepsUnloaded>>,
    database: Res<AssetDatabase>,
    actions: &WorldActions,
) {
    let mut tracked = HashSet::new();
    let registry = database.config().registry();
    for unloaded in unloaded.take() {
        for id in unloaded.ids {
            if tracked.insert(id) {
                let meta = match registry.get(id.ty()) {
                    Some(meta) => meta,
                    None => continue,
                };

                actions.add(meta.deps_unloaded(id));
            }
        }
    }
}

pub struct NotifyDepsUnloaded {
    ids: HashSet<AssetId>,
}

impl NotifyDepsUnloaded {
    pub fn new(ids: HashSet<AssetId>) -> Self {
        Self { ids }
    }
}

impl std::ops::Deref for NotifyDepsUnloaded {
    type Target = HashSet<AssetId>;

    fn deref(&self) -> &Self::Target {
        &self.ids
    }
}

impl std::ops::DerefMut for NotifyDepsUnloaded {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ids
    }
}

impl Event for NotifyDepsUnloaded {}

pub struct AssetImported<A: Asset> {
    pub path: AssetPath,
    pub id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetImported<A> {
    pub fn new(path: AssetPath, id: AssetId) -> Self {
        Self {
            path,
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetImported<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Imported(self.id));
        Some(())
    }
}

pub struct AssetLoaded<A: Asset> {
    id: AssetId,
    asset: A,
    dependencies: Option<Vec<AssetId>>,
    parent: Option<AssetId>,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A) -> Self {
        Self {
            id,
            asset,
            dependencies: None,
            parent: None,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetId>) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub fn with_parent(mut self, parent: AssetId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn dependencies(&self) -> Option<&[AssetId]> {
        self.dependencies.as_deref()
    }

    pub fn parent(&self) -> Option<AssetId> {
        self.parent
    }
}

impl<A: Asset> WorldAction for AssetLoaded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world.resource_mut::<Assets<A>>().add(self.id, self.asset);
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Loaded(self.id));

        let database = world.resource::<AssetDatabase>();
        let mut states = database.states.write_arc_blocking();
        Some(states.loaded(self.id, self.dependencies, self.parent))
    }
}

pub struct AssetDepsLoaded<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetDepsLoaded<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetDepsLoaded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::DepsLoaded(self.id));

        Some(())
    }
}

pub struct AssetDepsUnloaded<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetDepsUnloaded<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetDepsUnloaded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::DepsUnloaded(self.id));

        Some(())
    }
}

pub struct AssetUnloaded<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetUnloaded<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetUnloaded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let asset = world.resource_mut::<Assets<A>>().remove(&self.id);
        let database = world.resource::<AssetDatabase>();
        let mut states = database.states.write_arc_blocking();
        let state = states.unload(self.id)?;

        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Unloaded {
                id: self.id,
                asset,
                state,
            });

        Some(())
    }
}

pub struct AssetFailed<A: Asset> {
    id: AssetId,
    error: LoadError,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetFailed<A> {
    pub fn new(id: AssetId, error: impl Into<LoadError>) -> Self {
        Self {
            id,
            error: error.into(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetFailed<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let database = world.resource::<AssetDatabase>();
        let mut states = database.states.write_arc_blocking();
        states.failed(self.id);

        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Failed {
                id: self.id,
                error: self.error,
            });

        Some(())
    }
}

pub struct ReloadAssets {
    ids: Vec<AssetId>,
}

impl ReloadAssets {
    pub fn new(ids: Vec<AssetId>) -> Self {
        Self { ids }
    }
}

impl WorldAction for ReloadAssets {
    fn execute(mut self, world: &mut ecs::world::World) -> Option<()> {
        let database = world.resource::<AssetDatabase>();
        let states = database.states.read_arc_blocking();

        let reloads = self.ids.drain(..).filter_map(|id| {
            let state = states.load_state(id);
            (state.is_loaded() || state.is_failed()).then_some(AssetLoadPath::Id(id))
        });

        database.load(reloads);

        Some(())
    }
}

pub struct UnloadAssets {
    ids: Vec<AssetId>,
}

impl UnloadAssets {
    pub fn new(ids: Vec<AssetId>) -> Self {
        Self { ids }
    }
}

impl WorldAction for UnloadAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let config = world.resource::<AssetDatabase>().config();
        for id in self.ids {
            let meta = match config.registry().get(id.ty()) {
                Some(meta) => meta,
                None => continue,
            };

            world.actions().add(meta.unloaded(id));
        }

        Some(())
    }
}

pub struct ImportAssets {
    paths: Vec<AssetPath>,
}

impl ImportAssets {
    pub fn new(paths: Vec<AssetPath>) -> Self {
        Self { paths }
    }
}

impl WorldAction for ImportAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world.resource::<AssetDatabase>().import(self.paths);
        Some(())
    }
}
