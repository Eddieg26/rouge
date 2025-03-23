use asset::{AssetId, Assets, asset::Asset};
use ecs::{
    IndexSet, ResMut, Resource, ResourceId,
    event::{Event, Events},
    system::{
        AccessType, ArgItem, IntoSystemConfigs, StaticArg, SystemArg, SystemConfig, SystemFunc,
        WorldAccess,
    },
    world::action::{WorldAction, WorldActionFn, WorldActions},
};
use game::Main;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};

use super::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderAssetType(TypeId);
impl RenderAssetType {
    pub fn of<T: RenderAsset>() -> Self {
        Self(TypeId::of::<T>())
    }
}

pub trait RenderAsset: Send + Sync + 'static {}

pub struct RenderAssets<R: RenderAsset> {
    assets: HashMap<Id<R>, R>,
}

impl<R: RenderAsset> RenderAssets<R> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: Id<R>, asset: R) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: &Id<R>) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &Id<R>) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &Id<R>) -> Option<R> {
        self.assets.remove(id)
    }

    pub fn contains(&self, id: &Id<R>) -> bool {
        self.assets.contains_key(id)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, Id<R>, R> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, Id<R>, R> {
        self.assets.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Id<R>, &mut R) -> bool,
    {
        self.assets.retain(f);
    }

    pub fn drain(&mut self) -> std::collections::hash_map::Drain<'_, Id<R>, R> {
        self.assets.drain()
    }
}

impl<'a, R: RenderAsset> IntoIterator for &'a RenderAssets<R> {
    type Item = (&'a Id<R>, &'a R);
    type IntoIter = std::collections::hash_map::Iter<'a, Id<R>, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.assets.iter()
    }
}

impl<'a, R: RenderAsset> IntoIterator for &'a mut RenderAssets<R> {
    type Item = (&'a Id<R>, &'a mut R);
    type IntoIter = std::collections::hash_map::IterMut<'a, Id<R>, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.assets.iter_mut()
    }
}

impl<R: RenderAsset> Resource for RenderAssets<R> {}

pub struct ExtractInfo<R: RenderAssetExtractor> {
    pub extracted: Vec<(AssetId, R)>,
    pub removed: HashSet<Id<R::RenderAsset>>,
    pub re_extract: Vec<(AssetId, R)>,
}

impl<R: RenderAssetExtractor> ExtractInfo<R> {
    pub fn new() -> Self {
        Self {
            extracted: Vec::new(),
            removed: HashSet::new(),
            re_extract: Vec::new(),
        }
    }
}

impl<R: RenderAssetExtractor> Resource for ExtractInfo<R> {}

#[allow(unused_variables)]
pub trait RenderAssetExtractor: Asset + Clone {
    type RenderAsset: RenderAsset;
    type Arg: SystemArg;

    fn extract(
        asset: Self,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::RenderAsset, ExtractError<Self>>;

    fn removed(id: &AssetId, asset: &Self::RenderAsset, arg: &mut ArgItem<Self::Arg>) {}

    fn usage(asset: &Self) -> AssetUsage {
        AssetUsage::Discard
    }

    fn dependencies() -> Vec<RenderAssetType> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct AssetExtractors {
    pub(crate) registry: IndexSet<TypeId>,
    pub(crate) extract: Vec<SystemConfig>,
    pub(crate) process: Vec<SystemConfig>,
    pub(crate) dependencies: HashMap<TypeId, Vec<TypeId>>,
}

impl AssetExtractors {
    pub fn add<R: RenderAssetExtractor>(&mut self) -> bool {
        let ty = TypeId::of::<R>();
        if self.registry.contains(&ty) {
            return false;
        }

        let access = || {
            let mut access = R::Arg::access();
            access.push(WorldAccess::resource::<Assets<R>>(AccessType::Write));
            access.push(WorldAccess::resource::<RenderAssets<R::RenderAsset>>(
                AccessType::Write,
            ));
            access.push(WorldAccess::resource::<RenderAssetEvents<R>>(
                AccessType::Write,
            ));
            access.push(WorldAccess::resource::<Events<ExtractError<R>>>(
                AccessType::Write,
            ));

            for dep in R::dependencies() {
                access.push(WorldAccess::Resource {
                    ty: ResourceId::dynamic(dep.0.into()),
                    access: AccessType::Write,
                    send: true,
                });
            }

            access
        };

        let run: SystemFunc = Arc::new(|world| {
            let extract_info = ResMut::<ExtractInfo<R>>::get(world);
            let assets = ResMut::<RenderAssets<R::RenderAsset>>::get(world);
            let errors = ResMut::<Events<ExtractError<R>>>::get(world);
            let arg = StaticArg::<R::Arg>::get(world);

            Self::prepare(extract_info, assets, errors, arg);
        });

        let extract = Self::extractor::<R>.configs().remove(0);

        self.extract.push(extract);
        self.process
            .push(SystemConfig::new(None, run, access, true));

        let dependencies = R::dependencies();
        let first_dependent_index = self
            .registry
            .iter()
            .position(|ty| dependencies.iter().any(|dep| dep.0 == *ty));

        if let Some(index) = first_dependent_index {
            self.registry.insert_before(index, ty);
        } else {
            self.registry.insert(ty);
        }

        self.dependencies
            .insert(ty, dependencies.into_iter().map(|dep| dep.0).collect());

        true
    }

    fn extractor<R: RenderAssetExtractor>(
        mut assets: Main<ResMut<Assets<R>>>,
        mut extract_info: ResMut<ExtractInfo<R>>,
        mut events: Main<ResMut<RenderAssetEvents<R>>>,
    ) {
        assets.extend(extract_info.re_extract.drain(..));
        extract_info.removed.clear();

        for event in events.drain(..) {
            match event {
                RenderAssetEvent::Added(ref id) | RenderAssetEvent::Modified(ref id) => {
                    let asset = match assets.get_mut(id).cloned() {
                        Some(source) => source,
                        None => continue,
                    };

                    if R::usage(&asset) == AssetUsage::Discard {
                        assets.remove(id);
                    }

                    extract_info.extracted.push((*id, asset));
                }
                RenderAssetEvent::Removed(id) => {
                    extract_info.removed.insert(id.into());
                }
            }
        }
    }

    pub fn prepare<R: RenderAssetExtractor>(
        mut extract_info: ResMut<ExtractInfo<R>>,
        mut assets: ResMut<RenderAssets<R::RenderAsset>>,
        mut errors: ResMut<Events<ExtractError<R>>>,
        arg: StaticArg<R::Arg>,
    ) {
        let mut arg = arg.into_inner();
        let mut extract = vec![];

        for (id, asset) in extract_info.extracted.drain(..) {
            match R::extract(asset, &mut arg) {
                Ok(render_asset) => {
                    assets.add(id.into(), render_asset);
                }
                Err(ExtractError::Retry(asset)) => {
                    extract.push((id, asset));
                }
                Err(ExtractError::Error(error)) => {
                    errors.send(ExtractError::Error(error));
                }
            }
        }

        assets.retain(|id, _| !extract_info.removed.contains(&id));

        extract_info.re_extract = extract;
    }
}

impl Resource for AssetExtractors {}

pub enum RenderAssetEvent {
    Added(AssetId),
    Modified(AssetId),
    Removed(AssetId),
}

pub struct RenderAssetEvents<A: Asset> {
    events: Vec<RenderAssetEvent>,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Asset> RenderAssetEvents<A> {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn add(&mut self, event: RenderAssetEvent) {
        self.events.push(event);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, RenderAssetEvent> {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<A: Asset> std::ops::Deref for RenderAssetEvents<A> {
    type Target = Vec<RenderAssetEvent>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<A: Asset> std::ops::DerefMut for RenderAssetEvents<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

impl<A: Asset> Resource for RenderAssetEvents<A> {}

pub trait RenderResource: Resource + Send + Sync + Sized {
    type Arg: SystemArg;

    fn extract(arg: ArgItem<Self::Arg>) -> Result<Self, ExtractError<()>>;
}

pub struct ExtractResource<R: RenderResource>(std::marker::PhantomData<R>);
impl<R: RenderResource> ExtractResource<R> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<R: RenderResource> WorldAction for ExtractResource<R> {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let arg = R::Arg::get(unsafe { world.cell() });
        match R::extract(arg) {
            Ok(resource) => {
                world.add_resource(resource);
                None
            }
            Err(error) => {
                world.send_event(error);
                world.resource_mut::<ResourceExtractors>().add::<R>();
                None
            }
        }
    }
}

#[derive(Default)]
pub struct ResourceExtractors(HashMap<TypeId, WorldActionFn>);

impl ResourceExtractors {
    pub fn add<R: RenderResource>(&mut self) {
        self.0
            .insert(TypeId::of::<R>(), ExtractResource::<R>::new().into());
    }

    pub fn take(&mut self) -> Vec<WorldActionFn> {
        self.0.drain().map(|(_, action)| action).collect()
    }

    pub(crate) fn process(mut extractors: ResMut<ResourceExtractors>, actions: WorldActions) {
        actions.extend(extractors.take());
    }
}

impl Resource for ResourceExtractors {}


#[derive(Debug, Clone)]
pub enum ExtractError<T: Send + Sync + 'static = ()> {
    Retry(T),
    Error(Arc<dyn std::error::Error + Send + Sync + 'static>),
}

impl<T: Send + Sync + 'static> ExtractError<T> {
    pub fn from_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Error(Arc::new(error))
    }
}

impl<T: Send + Sync + 'static> Event for ExtractError<T> {}
