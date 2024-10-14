use super::{AssetEvent, StartAssetEvent};
use crate::{
    asset::{AssetId, AssetPath, ErasedAsset},
    cache::AssetCache,
    database::AssetDatabase,
    io::AssetIoError,
};
use ecs::{
    event::Events,
    task::AsyncTaskPool,
    world::{action::WorldAction, World},
};
use hashbrown::HashMap;
use std::error::Error;

#[derive(Debug)]
pub enum LoadError {
    AssetNotFound(AssetPath),
    MetadataNotFound(AssetPath),
    LoadArtifact {
        path: AssetPath,
        error: AssetIoError,
    },
    DeserializeError {
        path: AssetPath,
        error: bincode::Error,
    },
}
pub struct LoadRequest {
    pub path: AssetPath,
    pub load_dependencies: bool,
}

impl LoadRequest {
    pub fn new(path: AssetPath, load_dependencies: bool) -> Self {
        Self {
            path,
            load_dependencies,
        }
    }

    pub fn soft(path: AssetPath) -> Self {
        Self::new(path, false)
    }

    pub fn hard(path: AssetPath) -> Self {
        Self::new(path, true)
    }
}

pub struct LoadResult {
    pub id: AssetId,
    pub asset: ErasedAsset,
    pub dependencies: Vec<AssetId>,
}

impl LoadResult {
    pub fn new(id: AssetId, asset: ErasedAsset) -> Self {
        Self {
            id,
            asset,
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetId>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

pub struct LoadAssets {
    requests: Vec<LoadRequest>,
}

impl LoadAssets {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    pub fn push(&mut self, request: LoadRequest) {
        self.requests.push(request);
    }

    pub fn pop(&mut self) -> Option<LoadRequest> {
        self.requests.pop()
    }

    pub fn extend(&mut self, requests: impl IntoIterator<Item = LoadRequest>) {
        self.requests.extend(requests);
    }

    async fn load<'a>(
        request: &'a LoadRequest,
        database: &'a AssetDatabase,
        cache: &'a AssetCache,
    ) -> Result<LoadResult, LoadError> {
        let id = match &request.path {
            AssetPath::Id { id } => *id,
            AssetPath::Path { source, path } => {
                let library = database.library();
                let library = library.get_sources(source).unwrap();
                match library.get(path) {
                    Some(id) => id,
                    None => return Err(LoadError::AssetNotFound(request.path.clone())),
                }
            }
        };

        let metadata = match database.config().registry().get(&id.ty()) {
            Some(metadata) => metadata,
            None => return Err(LoadError::MetadataNotFound(request.path.clone())),
        };

        let artifact = match cache.load_artifact(id).await {
            Ok(artifact) => artifact,
            Err(error) => {
                return Err(LoadError::LoadArtifact {
                    path: request.path.clone(),
                    error,
                })
            }
        };

        let asset = match metadata.deserialize(artifact.asset()) {
            Ok(asset) => asset,
            Err(error) => {
                return Err(LoadError::DeserializeError {
                    path: request.path.clone(),
                    error,
                })
            }
        };

        let result = LoadResult::new(id, asset);

        match request.load_dependencies {
            true => Ok(result.with_dependencies(artifact.meta.dependencies)),
            false => Ok(result),
        }
    }
}

impl WorldAction for LoadAssets {
    fn execute(self, world: &mut World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(self.into()))
    }
}

impl AssetEvent for LoadAssets {
    fn execute(
        &mut self,
        database: &crate::database::AssetDatabase,
        actions: &ecs::world::action::WorldActions,
    ) {
        let cache = database.config().cache();
        let mut pool = AsyncTaskPool::new();
        // let mut loaded = HashMap::new();
        for requests in self.requests.chunks(100) {
            for request in requests {
                pool.spawn(Self::load(request, database, cache));
            }
        }
    }
}
