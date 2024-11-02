use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetType, Settings},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache, AssetInfo, AssetLoadPath, LoadedAsset},
        source::{AssetPath, AssetSource},
        AssetIoError, AssetReader,
    },
};
use ecs::event::Event;
use std::{error::Error, future::Future};

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
        let path = self.path.with_name(name.to_string());
        let meta = ArtifactMeta::new(id, path, 0).with_parent(*self.id());
        let asset = Artifact::from_asset(&asset, meta)?;
        self.sub_assets.push(asset);
        Ok(())
    }

    pub fn finish(self) -> (Vec<AssetId>, Vec<Artifact>) {
        (self.dependencies, self.sub_assets)
    }
}

pub trait Importer {
    type Asset: Asset;
    type Settings: Settings;
    type Processor: Processor<Asset = Self::Asset, Settings = Self::Settings>;
    type Error: Error + Send + Sync + 'static;

    fn import(
        ctx: &mut ImportContext<Self::Asset, Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;

    fn extensions() -> &'static [&'static str];
}

pub struct ProcessContext<'a, A: Asset, S: Settings> {
    cache: &'a AssetCache,
    metadata: &'a AssetMetadata<A, S>,
    dependencies: Vec<AssetInfo>,
}

impl<'a, A: Asset, S: Settings> ProcessContext<'a, A, S> {
    pub fn new(cache: &'a AssetCache, metadata: &'a AssetMetadata<A, S>) -> Self {
        Self {
            cache,
            metadata,
            dependencies: vec![],
        }
    }

    pub async fn load_asset<O: Asset>(
        &mut self,
        id: &AssetId,
    ) -> Result<LoadedAsset<O>, AssetIoError> {
        let asset = self.cache.load_asset::<O>(id).await?;
        self.dependencies
            .push(AssetInfo::new(*id, asset.meta().checksum));
        Ok(asset)
    }

    pub fn metadata(&self) -> &AssetMetadata<A, S> {
        self.metadata
    }

    pub fn finish(self) -> Vec<AssetInfo> {
        self.dependencies
    }
}

pub trait Processor {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn process<'a>(
        asset: &'a mut Self::Asset,
        ctx: &'a ProcessContext<Self::Asset, Self::Settings>,
    ) -> impl Future<Output = Result<(), Self::Error>> + 'a;
}

pub struct DefaultProcessor<A: Asset, S: Settings>(std::marker::PhantomData<(A, S)>);
impl<A: Asset, S: Settings> Processor for DefaultProcessor<A, S> {
    type Asset = A;
    type Settings = S;
    type Error = std::convert::Infallible;

    fn process<'a>(
        _asset: &'a mut Self::Asset,
        _ctx: &'a ProcessContext<Self::Asset, Self::Settings>,
    ) -> impl Future<Output = Result<(), Self::Error>> + 'a {
        async move { Ok(()) }
    }
}

#[derive(Debug)]
pub enum ImportError {
    MissingExtension {
        path: AssetPath,
    },
    MissingPath {
        id: AssetId,
    },
    InvalidSource {
        path: AssetPath,
    },
    InvalidExtension {
        path: AssetPath,
    },
    InvalidMeta {
        ty: AssetType,
    },
    MissingMainAsset {
        path: AssetPath,
    },
    NoProcessor {
        path: AssetPath,
    },
    Import {
        path: AssetPath,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    Process {
        path: AssetPath,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}

impl ImportError {
    pub fn import(
        path: AssetPath,
        error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        ImportError::Import {
            path,
            error: error.into(),
        }
    }

    pub fn process(
        path: AssetPath,
        error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        ImportError::Process {
            path,
            error: error.into(),
        }
    }

    pub fn path_or_id(&self) -> (Option<&AssetPath>, Option<AssetId>) {
        match self {
            ImportError::MissingExtension { path }
            | ImportError::InvalidSource { path }
            | ImportError::InvalidExtension { path }
            | ImportError::MissingMainAsset { path }
            | ImportError::NoProcessor { path }
            | ImportError::Import { path, .. }
            | ImportError::Process { path, .. } => (Some(path), None),
            ImportError::MissingPath { id } => (None, Some(*id)),
            ImportError::InvalidMeta { .. } => (None, None),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for ImportError {}
impl Event for ImportError {}

#[derive(Debug, Clone)]
pub enum LoadError {
    NotFound {
        path: AssetPath,
    },
    NotRegistered {
        ty: AssetType,
        path: AssetLoadPath,
    },
    Io {
        id: AssetId,
        error: AssetIoError,
        path: AssetLoadPath,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoadError::NotFound { path, .. } => write!(f, "Asset not found: {}", path),
            LoadError::NotRegistered { ty, .. } => write!(f, "Asset type not registered: {:?}", ty),
            LoadError::Io { id, error, .. } => {
                write!(f, "IO error loading asset {:?}: {}", id, error)
            }
        }
    }
}

impl Event for LoadError {}
impl std::error::Error for LoadError {}
