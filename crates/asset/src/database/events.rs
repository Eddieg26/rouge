use super::{state::AssetState, AssetDatabase};
use crate::{
    asset::{Asset, AssetId, Assets},
    importer::LoadError,
    io::{cache::AssetLoadPath, source::AssetPath},
};
use ecs::{
    event::{Event, Events},
    world::action::WorldAction,
};

pub enum AssetEvent<A: Asset> {
    Imported(AssetId),
    Loaded(AssetId),
    Unloaded {
        id: AssetId,
        asset: Option<A>,
        state: AssetState,
    },
    DepsLoaded(AssetId),
    Failed {
        id: AssetId,
        error: LoadError,
    },
}

impl<A: Asset> Event for AssetEvent<A> {}

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
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A) -> Self {
        Self {
            id,
            asset,
            dependencies: None,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetId>) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn dependencies(&self) -> Option<&[AssetId]> {
        self.dependencies.as_deref()
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
        Some(states.loaded(self.id, self.dependencies))
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
