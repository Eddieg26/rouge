use crate::{
    asset::{Asset, AssetId, AssetSettings, Settings},
    io::{
        cache::{Artifact, ArtifactMeta},
        source::AssetSource,
        AssetFuture, AssetReader,
    },
};
use ecs::event::Event;
use hashbrown::HashSet;
use std::{
    error::Error,
    path::{Path, PathBuf},
};

pub struct ImportContext<'a, S: Settings> {
    path: &'a Path,
    source: &'a AssetSource,
    settings: &'a AssetSettings<S>,
    dependencies: HashSet<AssetId>,
    sub_assets: Vec<Artifact>,
}

impl<'a, S: Settings> ImportContext<'a, S> {
    pub fn new(path: &'a Path, source: &'a AssetSource, settings: &'a AssetSettings<S>) -> Self {
        Self {
            path,
            source,
            settings,
            dependencies: HashSet::new(),
            sub_assets: vec![],
        }
    }

    pub fn path(&self) -> &Path {
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

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_sub_asset<A: Asset>(&mut self, id: AssetId, asset: A) -> Result<(), bincode::Error> {
        let mut meta = ArtifactMeta::new(id);
        meta.parent = Some(*self.id());
        let asset = Artifact::from_asset::<A>(&asset, meta)?;
        Ok(self.sub_assets.push(asset))
    }

    pub fn finish(self) -> (HashSet<AssetId>, Vec<Artifact>) {
        (self.dependencies, self.sub_assets)
    }
}

pub trait AssetImporter {
    type Asset: Asset;
    type Settings: Settings;
    type Processor: AssetProcessor<Asset = Self::Asset, Settings = Self::Settings>;
    type Error: Error + Send + Sync + 'static;

    fn import<'a>(
        ctx: &'a mut ImportContext<Self::Settings>,
        reader: &'a mut dyn AssetReader,
    ) -> AssetFuture<'a, Self::Asset, Self::Error>;

    fn extensions() -> &'static [&'static str];
}


pub struct ProcessContext<'a, S: Settings> {
    settings: &'a AssetSettings<S>,
    sub_assets: Vec<Artifact>,
    
}

pub trait AssetProcessor {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn process<'a>(
        ctx: &'a mut ImportContext<Self::Settings>,
        asset: &'a mut Self::Asset,
    ) -> AssetFuture<'a, (), Self::Error>;
}

#[derive(Debug)]
pub struct ImportError {
    path: PathBuf,
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl ImportError {
    pub fn from<E: std::error::Error + Send + Sync + 'static>(
        path: impl AsRef<Path>,
        error: E,
    ) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            error: Box::new(error),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }

    pub fn missing_extension(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing extension",
            )),
        }
    }

    pub fn unsupported_extension(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unsupported extension",
            )),
        }
    }

    pub fn missing_main(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            error: Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing main asset",
            )),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Import error for {}: {}",
            self.path.display(),
            self.error
        )
    }
}

impl<A: AsRef<Path>, E: std::error::Error + Send + Sync + 'static> From<(A, E)> for ImportError {
    fn from((path, error): (A, E)) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            error: Box::new(error),
        }
    }
}

impl std::error::Error for ImportError {}
impl Event for ImportError {}
