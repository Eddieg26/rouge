use super::{state::AssetState, AssetDatabase};
use crate::{
    asset::{Asset, AssetId, Assets},
    importer::LoadError,
    io::{cache::LoadPath, source::AssetPath},
};
use ecs::{
    core::resource::{Res, ResMut, Resource},
    event::{Event, Events},
    world::{action::WorldAction, builtin::events::ResourceUpdate},
};
use hashbrown::HashSet;

pub enum AssetEvent<A: Asset> {
    Imported {
        id: AssetId,
    },
    Added {
        id: AssetId,
    },
    Unloaded {
        id: AssetId,
        asset: Option<A>,
        state: AssetState,
    },
    Loaded {
        id: AssetId,
    },
    Modified {
        id: AssetId,
    },
    Failed {
        id: AssetId,
        error: LoadError,
    },
}

impl<A: Asset> Event for AssetEvent<A> {}

pub(crate) fn on_asset_event<A: Asset>(
    database: Res<AssetDatabase>,
    events: Res<Events<AssetEvent<A>>>,
    mut modified_list: ResMut<AssetsModified>,
    mut modified_events: ResMut<Events<ResourceUpdate<AssetsModified>>>,
) {
    for event in events.iter() {
        let id = match event {
            AssetEvent::Modified { id } => id,
            AssetEvent::Unloaded { id, .. } => id,
            _ => continue,
        };

        let states = database.states.read_arc_blocking();
        if let Some(state) = states.get(id) {
            for dep in state.dependents() {
                modified_list.add(*dep);
            }
        }
    }

    modified_events.add(ResourceUpdate::new());
}

pub(crate) fn on_update_assets_modified(
    database: Res<AssetDatabase>,
    mut modified_list: ResMut<AssetsModified>,
) {
    for id in modified_list.drain() {
        let meta = match database.config().registry().get(id.ty()) {
            Some(meta) => meta,
            None => continue,
        };

        meta.modified(id);
    }
}

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
            .add(AssetEvent::Imported { id: self.id });
        Some(())
    }
}

pub struct AssetAdded<A: Asset> {
    id: AssetId,
    asset: A,
    dependencies: Option<Vec<AssetId>>,
    parent: Option<AssetId>,
}

impl<A: Asset> AssetAdded<A> {
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

impl<A: Asset> WorldAction for AssetAdded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world.resource_mut::<Assets<A>>().add(self.id, self.asset);
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Added { id: self.id });

        let database = world.resource::<AssetDatabase>();
        let mut states = database.states.write_arc_blocking();
        states.loaded(self.id, self.dependencies, self.parent);
        Some(())
    }
}

pub struct AssetLoaded<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetLoaded<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Loaded { id: self.id });

        Some(())
    }
}

pub(crate) struct AssetModified<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetModified<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for AssetModified<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<AssetEvent<A>>>()
            .add(AssetEvent::Modified { id: self.id });

        Some(())
    }
}

pub struct AssetsModified {
    ids: HashSet<AssetId>,
}

impl AssetsModified {
    pub fn new() -> Self {
        Self {
            ids: HashSet::new(),
        }
    }

    pub fn add(&mut self, id: AssetId) {
        self.ids.insert(id);
    }

    pub fn remove(&mut self, id: AssetId) {
        self.ids.remove(&id);
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.ids.contains(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetId> {
        self.ids.iter()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = AssetId> + '_ {
        self.ids.drain()
    }
}

impl Resource for AssetsModified {}

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
            let state = states.get_load_state(id);
            (state.is_loaded() || state.is_failed()).then_some(LoadPath::Id(id))
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

pub struct LoadAsset<A: Asset> {
    pub path: LoadPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub fn new(path: impl Into<LoadPath>) -> Self {
        Self {
            path: path.into(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<A: Asset> WorldAction for LoadAsset<A> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let database = world.resource::<AssetDatabase>();
        Some(database.load(std::iter::once(self.path)))
    }
}

pub struct LoadAssets {
    paths: HashSet<LoadPath>,
}

impl LoadAssets {
    pub fn new(paths: HashSet<LoadPath>) -> Self {
        Self { paths }
    }
}

impl WorldAction for LoadAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let database = world.resource::<AssetDatabase>();
        Some(database.load(self.paths))
    }
}
