use asset::{AssetId, Assets, asset::Asset};
use ecs::{
    ResMut, Resource, ResourceId,
    event::{Event, Events},
    system::{AccessType, ArgItem, StaticArg, SystemArg, SystemConfig, SystemFunc, WorldAccess},
    world::action::{WorldAction, WorldActionFn},
};
use game::Main;
use std::{any::TypeId, collections::HashMap, hash::Hash, sync::Arc};

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

#[allow(unused_variables)]
pub trait RenderAssetExtractor: Asset {
    type RenderAsset: RenderAsset;
    type Arg: SystemArg;

    fn extract(
        id: &AssetId,
        asset: &mut Self,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::RenderAsset, ExtractError>;

    fn update(
        id: &AssetId,
        asset: &mut Self,
        render_asset: &mut Self::RenderAsset,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        Ok(())
    }

    fn removed(id: &AssetId, asset: &Self::RenderAsset, arg: &mut ArgItem<Self::Arg>) {}

    fn usage(id: &AssetId, asset: &Self) -> AssetUsage {
        AssetUsage::Discard
    }

    fn dependencies() -> &'static [RenderAssetType] {
        &[]
    }
}

pub struct AssetExtractors(HashMap<TypeId, SystemConfig>);

impl AssetExtractors {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add<R: RenderAssetExtractor>(&mut self) {
        let access = || {
            let mut access = R::Arg::access();
            access.push(WorldAccess::resource::<Assets<R>>(AccessType::Write));
            access.push(WorldAccess::resource::<RenderAssets<R::RenderAsset>>(
                AccessType::Write,
            ));
            access.push(WorldAccess::resource::<RenderAssetEvents<R>>(
                AccessType::Write,
            ));
            access.push(WorldAccess::resource::<Events<ExtractError>>(
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
            let assets = Main::<ResMut<Assets<R>>>::get(world);
            let render_assets = ResMut::<RenderAssets<R::RenderAsset>>::get(world);
            let events = Main::<ResMut<RenderAssetEvents<R>>>::get(world);
            let errors = Main::<ResMut<Events<ExtractError>>>::get(world);
            let arg = StaticArg::<R::Arg>::get(world);

            Self::extractor(assets, render_assets, events, errors, arg);
        });

        let config = SystemConfig::new(None, run, access, true);
        self.0.insert(TypeId::of::<R>(), config);
    }

    fn extractor<R: RenderAssetExtractor>(
        mut assets: Main<ResMut<Assets<R>>>,
        mut render_assets: ResMut<RenderAssets<R::RenderAsset>>,
        mut events: Main<ResMut<RenderAssetEvents<R>>>,
        mut errors: Main<ResMut<Events<ExtractError>>>,
        arg: StaticArg<R::Arg>,
    ) {
        let mut arg = arg.into_inner();
        let mut re_extract = vec![];

        for event in events.drain(..) {
            match event {
                RenderAssetEvent::Added(ref id) => {
                    let asset = match assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    match R::extract(id, asset, &mut arg) {
                        Ok(extracted) => {
                            if R::usage(id, asset) == AssetUsage::Discard {
                                assets.remove(id);
                            }

                            render_assets.add(id.into(), extracted);
                        }
                        Err(error) => {
                            errors.add(error);
                            re_extract.push(RenderAssetEvent::Added(*id));
                        }
                    }
                }
                RenderAssetEvent::Modified(ref id) => {
                    let asset = match assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    let render_asset = match render_assets.get_mut(&id.into()) {
                        Some(render_asset) => render_asset,
                        None => continue,
                    };

                    if let Err(error) = R::update(id, asset, render_asset, &mut arg) {
                        errors.add(error);
                        re_extract.push(RenderAssetEvent::Modified(*id));
                    } else if R::usage(id, asset) == AssetUsage::Discard {
                        assets.remove(id);
                        render_assets.remove(&id.into());
                    }
                }
                RenderAssetEvent::Removed(id) => {
                    if let Some(asset)  = render_assets.remove(&id.into()) {
                        R::removed(&id, &asset, &mut arg);
                    }
                }
            }
        }

        events.extend(re_extract);
    }

    pub fn take(self) -> HashMap<TypeId, SystemConfig> {
        self.0
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

    fn extract(arg: ArgItem<Self::Arg>) -> Result<Self, ExtractError>;
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
                Some(())
            }
            Err(error) => {
                world.send_event(error);
                Some(())
            }
        }
    }
}

pub struct ResourceExtractors(HashMap<TypeId, WorldActionFn>);

impl ResourceExtractors {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add<R: RenderResource>(&mut self) {
        self.0
            .insert(TypeId::of::<R>(), ExtractResource::<R>::new().into());
    }
}

impl Resource for ResourceExtractors {}

pub struct ExtractResources;
impl WorldAction for ExtractResources {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let resources = world.remove_resource::<ResourceExtractors>()?;
        for (_, action) in resources.0 {
            action.execute(world);
        }

        Some(())
    }
}

#[derive(Debug, Clone)]
pub enum ExtractError {
    MissingAsset,
    MissingDependency,
    Error(Arc<dyn std::error::Error + Send + Sync + 'static>),
}

impl ExtractError {
    pub fn from_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Error(Arc::new(error))
    }
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAsset => write!(f, "Missing asset"),
            Self::MissingDependency => write!(f, "Missing dependency"),
            Self::Error(error) => write!(f, "{}", error),
        }
    }
}

impl std::error::Error for ExtractError {}

impl Event for ExtractError {}
