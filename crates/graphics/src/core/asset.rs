use asset::{
    asset::{Asset, AssetId},
    Assets,
};
use ecs::{
    core::{
        resource::{Res, ResMut, Resource, ResourceId},
        IndexMap,
    },
    event::{Event, Events},
    system::{
        AccessType, ArgItem, IntoSystemConfigs, StaticArg, SystemArg, SystemConfig, WorldAccess,
    },
    world::{cell::WorldCell, World},
};
use game::Main;
use std::{collections::HashSet, hash::Hash, sync::Arc};

pub trait RenderAsset: Send + 'static {
    type Id: Copy + Eq + Hash + Send + 'static;

    fn world() -> RenderAssetWorld {
        RenderAssetWorld::Render
    }
}

pub struct RenderAssets<R: RenderAsset> {
    assets: IndexMap<R::Id, R>,
}

impl<R: RenderAsset> RenderAssets<R> {
    pub fn new() -> Self {
        Self {
            assets: IndexMap::new(),
        }
    }

    pub fn add(&mut self, id: R::Id, asset: R) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: &R::Id) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &R::Id) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &R::Id) -> Option<R> {
        self.assets.shift_remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&R::Id, &R)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&R::Id, &mut R)> {
        self.assets.iter_mut()
    }

    pub fn values(&self) -> impl Iterator<Item = &R> {
        self.assets.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut R> {
        self.assets.values_mut()
    }

    pub fn contains(&self, id: R::Id) -> bool {
        self.assets.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn sort_by(&mut self, sorter: impl FnMut(&R::Id, &R, &R::Id, &R) -> std::cmp::Ordering) {
        self.assets.sort_by(sorter);
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

impl<R: RenderAsset<Id: Ord>> RenderAssets<R> {
    pub fn sort_keys(&mut self) {
        self.assets.sort_keys();
    }
}

impl<R: RenderAsset> std::ops::Index<usize> for RenderAssets<R> {
    type Output = R;

    fn index(&self, index: usize) -> &Self::Output {
        &self.assets[index]
    }
}

impl<R: RenderAsset> std::ops::IndexMut<usize> for RenderAssets<R> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.assets[index]
    }
}

impl<R: RenderAsset> Default for RenderAssets<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RenderAsset> Resource for RenderAssets<R> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderAssetAction<A: Asset> {
    Added { id: AssetId },
    Modified { id: AssetId },
    Removed { id: AssetId },

    _Phantom(std::marker::PhantomData<A>),
}

pub struct RenderAssetActions<A: Asset> {
    actions: Vec<RenderAssetAction<A>>,
}

impl<A: Asset> RenderAssetActions<A> {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn add(&mut self, action: RenderAssetAction<A>) {
        self.actions.push(action);
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderAssetAction<A>> {
        self.actions.iter()
    }

    pub fn retain(&mut self, mut f: impl FnMut(&RenderAssetAction<A>) -> bool) {
        self.actions.retain(&mut f);
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn clear(&mut self) {
        self.actions.clear();
    }
}

impl<A: Asset> Resource for RenderAssetActions<A> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAssetWorld {
    Main,
    Render,
}

#[derive(Debug, Clone)]
pub enum ExtractError {
    MissingAsset,
    MissingDependency,
    DependencyFailed,
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
            Self::DependencyFailed => write!(f, "Dependency failed"),
            Self::Error(error) => write!(f, "{}", error),
        }
    }
}

impl std::error::Error for ExtractError {}

impl Event for ExtractError {}

#[allow(unused_variables)]
pub trait RenderAssetExtractor: 'static {
    type Source: Asset;
    type Asset: RenderAsset<Id: From<AssetId>>;
    type Arg: SystemArg;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, ExtractError>;

    fn update(
        id: &AssetId,
        source: &mut Self::Source,
        asset: &mut Self::Asset,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        Ok(())
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Asset>, arg: &mut ArgItem<Self::Arg>);

    fn usage(id: &AssetId, source: &Self::Source) -> AssetUsage {
        AssetUsage::Keep
    }
}

pub struct RenderAssetExtractors {
    extractors: IndexMap<ResourceId, Vec<SystemConfig>>,
    dependencies: IndexMap<ResourceId, HashSet<ResourceId>>,
}

impl RenderAssetExtractors {
    pub fn new() -> Self {
        Self {
            extractors: IndexMap::new(),
            dependencies: IndexMap::new(),
        }
    }

    pub fn add<R: RenderAssetExtractor>(&mut self) {
        let configs = match R::Asset::world() {
            RenderAssetWorld::Main => Self::extract_render_asset_main::<R>.configs(),
            RenderAssetWorld::Render => Self::extract_render_asset_render::<R>.configs(),
        };

        self.extractors
            .entry(ResourceId::of::<RenderAssets<R::Asset>>())
            .or_default()
            .extend(configs);
    }

    pub fn add_dependency<R: RenderAssetExtractor, D: RenderAssetExtractor>(&mut self) {
        self.dependencies
            .entry(ResourceId::of::<RenderAssets<R::Asset>>())
            .or_default()
            .insert(ResourceId::of::<RenderAssets<D::Asset>>());
    }

    pub fn build(mut self) -> Vec<SystemConfig> {
        for deps in self.dependencies.values() {
            for dep in deps {
                let configs = match self.extractors.get_mut(dep) {
                    Some(configs) => configs,
                    None => continue,
                };

                for config in configs {
                    config.add_custom(WorldAccess::Resource {
                        ty: *dep,
                        access: AccessType::Read,
                        send: true,
                    });
                }
            }
        }

        self.extractors.into_values().flatten().collect()
    }

    fn extract_render_asset_main<R: RenderAssetExtractor>(
        mut source_assets: Main<ResMut<Assets<R::Source>>>,
        mut assets: Main<ResMut<RenderAssets<R::Asset>>>,
        mut errors: Main<ResMut<Events<ExtractError>>>,
        actions: Main<Res<RenderAssetActions<R::Source>>>,
        arg: StaticArg<R::Arg>,
    ) {
        Self::extract_render_asset_inner::<R>(
            &mut source_assets,
            &mut assets,
            &mut errors,
            &actions,
            arg,
        );
    }

    fn extract_render_asset_render<R: RenderAssetExtractor>(
        mut source_assets: Main<ResMut<Assets<R::Source>>>,
        mut assets: ResMut<RenderAssets<R::Asset>>,
        mut errors: Main<ResMut<Events<ExtractError>>>,
        actions: Main<Res<RenderAssetActions<R::Source>>>,
        arg: StaticArg<R::Arg>,
    ) {
        Self::extract_render_asset_inner::<R>(
            &mut source_assets,
            &mut assets,
            &mut errors,
            &actions,
            arg,
        );
    }

    fn extract_render_asset_inner<R: RenderAssetExtractor>(
        source_assets: &mut Assets<R::Source>,
        assets: &mut RenderAssets<R::Asset>,
        errors: &mut Events<ExtractError>,
        actions: &RenderAssetActions<R::Source>,
        arg: StaticArg<R::Arg>,
    ) {
        let mut arg = arg.into_inner();

        for action in actions.iter() {
            match action {
                RenderAssetAction::Added { id } => {
                    let source = match source_assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    match R::extract(id, source, &mut arg) {
                        Ok(asset) => {
                            let id = <R::Asset as RenderAsset>::Id::from(*id);
                            assets.add(id, asset);
                        }
                        Err(e) => errors.add(e),
                    };
                }
                RenderAssetAction::Modified { id } => {
                    let source = match source_assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    let asset = match assets.get_mut(&<R::Asset as RenderAsset>::Id::from(*id)) {
                        Some(asset) => asset,
                        None => continue,
                    };

                    if let Err(e) = R::update(id, source, asset, &mut arg) {
                        errors.add(e);
                    }
                }
                RenderAssetAction::Removed { id } => R::remove(id, assets, &mut arg),
                _ => continue,
            }
        }
    }
}

impl Resource for RenderAssetExtractors {}

pub trait RenderResourceExtractor: Resource + Send + Sized + 'static {
    type Arg: SystemArg;

    fn can_extract(world: &World) -> bool;
    fn extract(arg: ArgItem<Self::Arg>) -> Result<Self, ExtractError>;

    fn default() -> Option<Self> {
        None
    }
}

pub struct ResourceExtractor {
    extract: Box<dyn Fn(WorldCell) -> bool + Send + Sync>,
}

impl ResourceExtractor {
    pub fn new<R: RenderResourceExtractor>() -> Self {
        Self {
            extract: Box::new(|world| match R::can_extract(world.get()) {
                true => {
                    let arg = R::Arg::get(world);
                    match R::extract(arg) {
                        Ok(resource) => {
                            world.get_mut().add_resource(resource);
                        }
                        Err(e) => {
                            world.resource_mut::<Events<ExtractError>>().add(e);
                        }
                    }

                    true
                }
                false => false,
            }),
        }
    }

    pub fn extract(&self, world: WorldCell) -> bool {
        (self.extract)(world)
    }
}

#[derive(Default)]
pub struct RenderResourceExtractors {
    extractors: IndexMap<ResourceId, ResourceExtractor>,
}

impl RenderResourceExtractors {
    pub fn new() -> Self {
        Self {
            extractors: IndexMap::new(),
        }
    }

    pub fn add<R: RenderResourceExtractor>(&mut self) {
        let id = ResourceId::of::<R>();
        if !self.extractors.contains_key(&id) {
            self.extractors.insert(id, ResourceExtractor::new::<R>());
        }
    }

    pub fn extract(&mut self, world: &World) {
        let world = unsafe { world.cell() };
        self.extractors
            .retain(|_, extractor| !extractor.extract(world));
    }
}

impl Resource for RenderResourceExtractors {}
