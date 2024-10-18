use crate::{
    asset::{Asset, AssetId, AssetMetadata, Settings},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache, LoadedAsset},
        source::{AssetPath, AssetSource},
        AssetFuture, AssetIoError, AssetReader,
    },
};
use ecs::event::Event;
use std::error::Error;

pub struct ImportContext<'a, A: Asset, S: Settings> {
    path: &'a AssetPath,
    source: &'a AssetSource,
    settings: &'a AssetMetadata<A, S>,
    dependencies: Vec<AssetId>,
    sub_assets: Vec<Artifact>,
}

impl<'a, A: Asset, S: Settings> ImportContext<'a, A, S> {
    pub fn new(
        path: &'a AssetPath,
        source: &'a AssetSource,
        settings: &'a AssetMetadata<A, S>,
    ) -> Self {
        Self {
            path,
            source,
            settings,
            dependencies: vec![],
            sub_assets: vec![],
        }
    }

    pub fn path(&self) -> &AssetPath {
        self.path
    }

    pub fn id(&self) -> &AssetId {
        self.settings.id()
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn source(&self) -> &AssetSource {
        &self.source
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }

    pub fn add_sub_asset<Sub: Asset>(
        &mut self,
        id: AssetId,
        name: impl ToString,
        asset: Sub,
    ) -> Result<(), bincode::Error> {
        let meta =
            ArtifactMeta::new(id, self.path.with_name(name.to_string())).with_parent(*self.id());
        let asset = Artifact::from_asset(&asset, meta)?;
        Ok(self.sub_assets.push(asset))
    }

    pub fn finish(self) -> (Vec<AssetId>, Vec<Artifact>) {
        (self.dependencies, self.sub_assets)
    }
}

pub trait AssetImporter {
    type Asset: Asset;
    type Settings: Settings;
    type Processor: AssetProcessor<Asset = Self::Asset, Settings = Self::Settings>;
    type Error: Error + Send + Sync + 'static;

    fn import<'a>(
        ctx: &'a mut ImportContext<Self::Asset, Self::Settings>,
        reader: &'a mut dyn AssetReader,
    ) -> AssetFuture<'a, Self::Asset, Self::Error>;

    fn extensions() -> &'static [&'static str];
}

pub struct ProcessContext<'a, A: Asset, S: Settings> {
    cache: &'a AssetCache,
    metadata: &'a AssetMetadata<A, S>,
}

impl<'a, A: Asset, S: Settings> ProcessContext<'a, A, S> {
    pub fn new(cache: &'a AssetCache, metadata: &'a AssetMetadata<A, S>) -> Self {
        Self { cache, metadata }
    }

    pub async fn load_asset<O: Asset>(&self, id: &AssetId) -> Result<LoadedAsset<O>, AssetIoError> {
        self.cache.load_asset(id).await
    }

    pub fn metadata(&self) -> &AssetMetadata<A, S> {
        self.metadata
    }
}

pub trait AssetProcessor {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn process<'a>(
        asset: &'a mut Self::Asset,
        ctx: &'a ProcessContext<Self::Asset, Self::Settings>,
    ) -> AssetFuture<'a, (), Self::Error>;
}

pub struct DefaultProcessor<A: Asset, S: Settings>(std::marker::PhantomData<(A, S)>);
impl<A: Asset, S: Settings> AssetProcessor for DefaultProcessor<A, S> {
    type Asset = A;
    type Settings = S;
    type Error = std::convert::Infallible;

    fn process<'a>(
        _: &'a mut Self::Asset,
        _: &'a ProcessContext<Self::Asset, Self::Settings>,
    ) -> AssetFuture<'a, (), Self::Error> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug)]
pub struct ImportError {
    path: AssetPath,
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl ImportError {
    pub fn from<E: std::error::Error + Send + Sync + 'static>(
        path: impl Into<AssetPath>,
        error: E,
    ) -> Self {
        Self {
            path: path.into(),
            error: Box::new(error),
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }

    pub fn missing_extension(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing extension",
            )),
        }
    }

    pub fn unsupported_extension(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unsupported extension",
            )),
        }
    }

    pub fn missing_main(path: impl Into<AssetPath>) -> Self {
        Self {
            path: path.into(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing main asset",
            )),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Import error for {:?}: {}", self.path, self.error)
    }
}

impl<A: Into<AssetPath>, E: std::error::Error + Send + Sync + 'static> From<(A, E)>
    for ImportError
{
    fn from((path, error): (A, E)) -> Self {
        Self {
            path: path.into(),
            error: Box::new(error),
        }
    }
}

impl std::error::Error for ImportError {}
impl Event for ImportError {}

#[derive(Debug)]
pub struct ProcessError {
    id: AssetId,
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl ProcessError {
    pub fn new(
        id: AssetId,
        error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            id,
            error: error.into(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to process asset: {:?}", self.id)
    }
}

impl std::error::Error for ProcessError {}
