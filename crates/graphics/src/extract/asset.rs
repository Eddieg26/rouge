use super::{AssetUsage, ExtractError, RenderAssetWorld};
use asset::{asset::Asset, AssetId, Assets};
use ecs::{
    core::{
        resource::{ResMut, Resource, ResourceId},
        IndexMap, IndexSet,
    },
    event::Events,
    system::{
        AccessType, ArgItem, IntoSystemConfigs, StaticArg, SystemArg, SystemConfig, WorldAccess,
    },
};
use game::Main;
use std::{collections::HashSet, hash::Hash};

pub trait RenderAsset: Send + 'static {
    type Id: Copy + Eq + Hash + Send + 'static;

    fn world() -> RenderAssetWorld {
        RenderAssetWorld::Render
    }
}

#[allow(unused_variables)]
pub trait RenderAssetExtractor: Asset {
    type Target: RenderAsset<Id: From<AssetId>>;
    type Arg: SystemArg;

    fn extract(
        id: &AssetId,
        asset: &mut Self,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Target, ExtractError>;

    fn update(
        id: &AssetId,
        asset: &mut Self,
        target: &mut Self::Target,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        Ok(())
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>, arg: &mut ArgItem<Self::Arg>);

    fn usage(id: &AssetId, asset: &Self) -> AssetUsage {
        AssetUsage::Keep
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

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
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

    pub fn extend(&mut self, actions: impl IntoIterator<Item = RenderAssetAction<A>>) {
        self.actions.extend(actions);
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
        let configs = match R::Target::world() {
            RenderAssetWorld::Main => Self::extract_render_asset_main::<R>.configs(),
            RenderAssetWorld::Render => Self::extract_render_asset_render::<R>.configs(),
        };

        self.extractors
            .entry(ResourceId::of::<RenderAssets<R::Target>>())
            .or_default()
            .extend(configs);
    }

    pub fn add_dependency<R: RenderAssetExtractor, D: RenderAssetExtractor>(&mut self) {
        self.dependencies
            .entry(ResourceId::of::<RenderAssets<R::Target>>())
            .or_default()
            .insert(ResourceId::of::<RenderAssets<D::Target>>());
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
        mut source_assets: Main<ResMut<Assets<R>>>,
        mut extract_assets: ResMut<ReExtractAssets<R>>,
        mut assets: Main<ResMut<RenderAssets<R::Target>>>,
        mut errors: Main<ResMut<Events<ExtractError>>>,
        mut actions: Main<ResMut<RenderAssetActions<R>>>,
        arg: StaticArg<R::Arg>,
    ) {
        Self::extract_render_asset_inner::<R>(
            &mut source_assets,
            &mut extract_assets,
            &mut assets,
            &mut errors,
            &mut actions,
            arg,
        );
    }

    fn extract_render_asset_render<R: RenderAssetExtractor>(
        mut source_assets: Main<ResMut<Assets<R>>>,
        mut extract_assets: ResMut<ReExtractAssets<R>>,
        mut assets: ResMut<RenderAssets<R::Target>>,
        mut errors: Main<ResMut<Events<ExtractError>>>,
        mut actions: Main<ResMut<RenderAssetActions<R>>>,
        arg: StaticArg<R::Arg>,
    ) {
        Self::extract_render_asset_inner::<R>(
            &mut source_assets,
            &mut extract_assets,
            &mut assets,
            &mut errors,
            &mut actions,
            arg,
        );
    }

    fn extract_render_asset_inner<R: RenderAssetExtractor>(
        source_assets: &mut Assets<R>,
        extract_assets: &mut ReExtractAssets<R>,
        assets: &mut RenderAssets<R::Target>,
        errors: &mut Events<ExtractError>,
        actions: &mut RenderAssetActions<R>,
        arg: StaticArg<R::Arg>,
    ) {
        let mut arg = arg.into_inner();
        let _ = extract_assets.assets.drain(..).map(|id| {
            actions.add(RenderAssetAction::Added { id });
        });

        for action in actions.iter() {
            match action {
                RenderAssetAction::Added { id } => {
                    let source = match source_assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    match R::extract(id, source, &mut arg) {
                        Ok(asset) => {
                            if R::usage(id, source) == AssetUsage::Discard {
                                source_assets.remove(id);
                            }

                            let id = <R::Target as RenderAsset>::Id::from(*id);
                            assets.add(id, asset);
                        }
                        Err(e) => {
                            errors.add(e);
                            extract_assets.assets.insert(*id);
                        }
                    };
                }
                RenderAssetAction::Modified { id } => {
                    let source = match source_assets.get_mut(id) {
                        Some(source) => source,
                        None => continue,
                    };

                    let asset = match assets.get_mut(&<R::Target as RenderAsset>::Id::from(*id)) {
                        Some(asset) => asset,
                        None => continue,
                    };

                    match R::update(id, source, asset, &mut arg) {
                        Ok(_) if R::usage(id, source) == AssetUsage::Discard => {
                            source_assets.remove(id);
                        }
                        Err(e) => errors.add(e),
                        _ => {}
                    }
                }
                RenderAssetAction::Removed { id } => R::remove(id, assets, &mut arg),
                _ => continue,
            }
        }
    }
}

impl Resource for RenderAssetExtractors {}

/// Resource used to re-extract assets every frame.
#[derive(Default)]
pub struct ReExtractAssets<A: Asset> {
    assets: IndexSet<AssetId>,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Asset> Resource for ReExtractAssets<A> {}
