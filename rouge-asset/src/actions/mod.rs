use self::{meta::AssetLoaderMetas, observers::on_import_folder};
use crate::{
    config::AssetConfig,
    database::{AssetDatabase, LoadState},
    loader::AssetLoader,
    storage::{AssetSettings, Assets},
    Asset, AssetId, AssetPath, Settings,
};
use rouge_ecs::{
    macros::Resource,
    observer::{Action, Actions, Observers},
    process::{Process, StartProcess},
    World,
};
use rouge_game::game::GameEnvironment;
use std::path::{Path, PathBuf};

pub mod meta;
pub mod observers;

#[derive(Clone, Resource)]
pub struct MainWorldActions(Actions);

impl MainWorldActions {
    pub fn new(actions: Actions) -> Self {
        MainWorldActions(actions)
    }
}

impl std::ops::Deref for MainWorldActions {
    type Target = Actions;

    fn deref(&self) -> &Actions {
        &self.0
    }
}

impl std::ops::DerefMut for MainWorldActions {
    fn deref_mut(&mut self) -> &mut Actions {
        &mut self.0
    }
}

pub struct ImportAssets<A: Asset> {
    paths: Vec<PathBuf>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ImportAssets<A> {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for ImportAssets<A> {
    type Output = Vec<PathBuf>;

    fn execute(&mut self, _: &mut rouge_ecs::World) -> Self::Output {
        std::mem::take(&mut self.paths)
    }
}

pub struct ImportFolder {
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Action for ImportFolder {
    type Output = PathBuf;

    fn execute(&mut self, _: &mut rouge_ecs::World) -> Self::Output {
        let path = std::mem::take(&mut self.path);

        println!("Importing folder: {:?}", &path);
        path
    }
}

pub struct LoadAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn process(path: impl Into<AssetPath>) -> StartProcess {
        StartProcess::new(LoadProcess::<A>::new(path.into()))
    }
}

impl<A: Asset> Action for LoadAsset<A> {
    type Output = AssetId;

    fn skip(&self, world: &rouge_ecs::World) -> bool {
        match &self.path {
            AssetPath::Id(id) => {
                let state = world.resource::<AssetDatabase>().load_state(*id);
                state == LoadState::Loaded || state == LoadState::Loading
            }
            AssetPath::Path(path) => {
                let db = world.resource::<AssetDatabase>();
                match db.id_from_path(Path::new(path)) {
                    Some(id) => {
                        let state = db.load_state(id);
                        state == LoadState::Loaded || state == LoadState::Loading
                    }
                    None => false,
                }
            }
        }
    }

    fn execute(&mut self, world: &mut rouge_ecs::World) -> Self::Output {
        match &self.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => {
                let db = world.resource_mut::<AssetDatabase>();
                match db.id_from_path(Path::new(path)) {
                    Some(id) => id,
                    None => {
                        let config = world.resource::<AssetConfig>();
                        let full_path = config.asset_path().join(path);
                        let info = config
                            .load_asset_info::<A>(&full_path)
                            .expect("Failed to load asset info.");

                        db.set_path_id(info.id(), &path);

                        info.id()
                    }
                }
            }
        }
    }
}

pub struct UnloadAsset<A: Asset> {
    path: AssetPath,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> UnloadAsset<A> {
    pub fn new(path: AssetPath) -> Self {
        Self {
            path,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for UnloadAsset<A> {
    type Output = AssetId;

    fn skip(&self, world: &rouge_ecs::World) -> bool {
        match &self.path {
            AssetPath::Id(id) => {
                let state = world.resource::<AssetDatabase>().load_state(*id);
                state == LoadState::Unloaded || state == LoadState::Unloading
            }
            AssetPath::Path(path) => {
                let db = world.resource::<AssetDatabase>();
                match db.id_from_path(Path::new(path)) {
                    Some(id) => {
                        let state = db.load_state(id);
                        state == LoadState::Unloaded || state == LoadState::Unloading
                    }
                    None => true,
                }
            }
        }
    }

    fn execute(&mut self, world: &mut rouge_ecs::World) -> Self::Output {
        match &self.path {
            AssetPath::Id(id) => *id,
            AssetPath::Path(path) => {
                let db = world.resource_mut::<AssetDatabase>();
                match db.id_from_path(Path::new(path)) {
                    Some(id) => id,
                    None => panic!("Failed to find asset id from path."),
                }
            }
        }
    }
}

pub struct ProcessAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ProcessAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for ProcessAsset<A> {
    type Output = AssetId;

    fn skip(&self, world: &rouge_ecs::World) -> bool {
        let db = world.resource::<AssetDatabase>();
        let state = db.load_state(self.id);
        state != LoadState::Loaded
    }

    fn execute(&mut self, _: &mut rouge_ecs::World) -> Self::Output {
        self.id
    }
}

pub struct AssetLoaded<A: Asset> {
    id: AssetId,
    asset: Option<A>,
}

impl<A: Asset> AssetLoaded<A> {
    pub fn new(id: AssetId, asset: A) -> Self {
        Self {
            id,
            asset: Some(asset),
        }
    }
}

impl<A: Asset> Action for AssetLoaded<A> {
    type Output = AssetId;

    fn skip(&self, world: &World) -> bool {
        world.resource::<Assets<A>>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::World) -> Self::Output {
        world
            .resource_mut::<Assets<A>>()
            .insert(self.id, self.asset.take().unwrap());
        self.id
    }
}

pub struct SettingsLoaded<S: Settings> {
    id: AssetId,
    settings: Option<S>,
}

impl<S: Settings> SettingsLoaded<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self {
            id,
            settings: Some(settings),
        }
    }
}

impl<S: Settings> Action for SettingsLoaded<S> {
    type Output = AssetId;

    fn skip(&self, world: &World) -> bool {
        world.resource::<AssetSettings<S>>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::World) -> Self::Output {
        world
            .resource_mut::<AssetSettings<S>>()
            .insert(self.id, self.settings.take().unwrap());
        self.id
    }
}

pub struct LoadProcess<A: Asset> {
    action: Option<LoadAsset<A>>,
    _ty: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadProcess<A> {
    pub fn new(path: impl Into<AssetPath>) -> Self {
        LoadProcess {
            action: Some(LoadAsset::new(path)),
            _ty: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Process for LoadProcess<A> {
    fn init(&mut self, main: &mut World, sub: &mut World) {
        let main_actions = MainWorldActions::new(main.actions().clone());
        let metas = main.resource::<AssetLoaderMetas>();
        for (_, meta) in metas.iter() {
            meta.add_on_import_observer(sub);
            meta.add_on_load_observer(sub);
            meta.add_on_unload_observer(sub);
            meta.add_on_process_observer(sub);
            meta.clone_cacher(main, sub);
        }
        sub.add_resource(metas.clone());
        sub.add_resource(main_actions);
        sub.add_resource(main.resource::<AssetDatabase>().clone());
        sub.add_resource(main.resource::<GameEnvironment>().clone());
        sub.add_resource(main.resource::<AssetConfig>().clone());
        sub.actions_mut().add(self.action.take().unwrap())
    }

    fn execute(&mut self, world: &mut World) {
        while !world.resource::<AssetDatabase>().is_imported()
            && world.resource::<GameEnvironment>().is_development()
        {}

        world.flush();
    }
}

pub struct ImportProcess {
    paths: Vec<PathBuf>,
}

impl ImportProcess {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

impl Process for ImportProcess {
    fn init(&mut self, main: &mut World, sub: &mut World) {
        let main_actions = MainWorldActions::new(main.actions().clone());
        let metas = main.resource::<AssetLoaderMetas>();
        for (_, meta) in metas.iter() {
            meta.add_on_import_observer(sub);
            meta.add_on_load_observer(sub);
            meta.add_on_unload_observer(sub);
            meta.add_on_process_observer(sub);
            meta.clone_cacher(main, sub);
        }

        sub.add_resource(main_actions);
        sub.add_resource(metas.clone());
        sub.add_resource(main.resource::<AssetDatabase>().clone());
        sub.add_resource(main.resource::<GameEnvironment>().clone());
        sub.add_resource(main.resource::<AssetConfig>().clone());
        sub.add_observers(Observers::<ImportFolder>::new().add_system(on_import_folder));
        for path in &self.paths {
            sub.actions_mut().add(ImportFolder::new(path.clone()));
        }
    }

    fn execute(&mut self, world: &mut World) {
        world.flush();

        world.resource_mut::<AssetDatabase>().set_imported();
        println!("Assets imported.");
    }
}
