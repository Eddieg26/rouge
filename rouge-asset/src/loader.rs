use crate::{error::AssetError, Asset, AssetDependency, AssetId, AssetMetadata, Settings};
use rouge_ecs::{bits::AsBytes, macros::Resource, ArgItem, SystemArg};
use std::path::Path;

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetDependency>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, metadata: &'a AssetMetadata<S>) -> Self {
        Self {
            path,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }

    pub fn add_dependency(&mut self, id: AssetDependency) {
        self.dependencies.push(id);
    }

    pub fn dependencies(&self) -> &[AssetDependency] {
        &self.dependencies
    }

    pub(crate) fn dependiencies_mut(&mut self) -> &mut Vec<AssetDependency> {
        &mut self.dependencies
    }
}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Arg: SystemArg;

    fn load(
        context: &mut LoadContext<Self::Settings>,
        data: &[u8],
    ) -> Result<Self::Asset, AssetError>;

    fn unload(
        _asset: &Self::Asset,
        _metadata: &AssetMetadata<Self::Settings>,
        _arg: &ArgItem<Self::Arg>,
    ) -> Result<(), AssetError> {
        Ok(())
    }
    fn process(
        _asset: &Self::Asset,
        _id: &AssetId,
        _settings: &Self::Settings,
        _arg: &ArgItem<Self::Arg>,
    ) -> Result<(), AssetError> {
        Ok(())
    }

    fn extensions() -> &'static [&'static str];
}

pub trait AssetRW: 'static {
    type Asset: Asset;

    fn read(asset: &Self::Asset) -> Vec<u8>;
    fn write(bytes: &[u8]) -> Option<Self::Asset>;
}

impl<A: Asset + AsBytes> AssetRW for A {
    type Asset = A;

    fn read(asset: &Self::Asset) -> Vec<u8> {
        asset.to_bytes()
    }

    fn write(bytes: &[u8]) -> Option<Self::Asset> {
        A::from_bytes(bytes)
    }
}

#[derive(Resource)]
pub struct AssetCacher<A: Asset> {
    cache: fn(path: &Path, &A, &[AssetDependency]) -> Result<(), AssetError>,
    load: fn(path: &Path) -> Result<(A, Vec<AssetDependency>), AssetError>,
}

impl<A: Asset> AssetCacher<A> {
    pub fn new<RW: AssetRW<Asset = A>>() -> Self {
        Self {
            cache: |path, asset, dependencies: &[AssetDependency]| {
                let asset = RW::read(asset);
                let asset_len = asset.len().to_bytes();
                let dependencies = dependencies.to_vec().to_bytes();
                let bytes = [asset_len, asset, dependencies].concat();
                std::fs::write(path, &bytes).map_err(|_| AssetError::InvalidData)
            },
            load: |path| {
                let bytes = std::fs::read(path).map_err(|_| AssetError::InvalidData)?;
                let (asset_len, bytes) = bytes.split_at(std::mem::size_of::<usize>());
                let asset_len = usize::from_bytes(asset_len).ok_or(AssetError::InvalidData)?;
                let (asset, dependencies) = bytes.split_at(asset_len);
                let asset = RW::write(asset).ok_or(AssetError::InvalidData)?;
                let dependencies = Vec::<AssetDependency>::from_bytes(dependencies)
                    .ok_or(AssetError::InvalidData)?;

                Ok((asset, dependencies))
            },
        }
    }

    pub fn cache(
        &self,
        path: &Path,
        asset: &A,
        dependencies: &[AssetDependency],
    ) -> Result<(), AssetError> {
        (self.cache)(path, asset, dependencies)
    }

    pub fn load(&self, path: &Path) -> Result<(A, Vec<AssetDependency>), AssetError> {
        (self.load)(path)
    }
}

impl<A: Asset> Clone for AssetCacher<A> {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            load: self.load.clone(),
        }
    }
}
