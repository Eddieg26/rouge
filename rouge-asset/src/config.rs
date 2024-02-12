use rouge_ecs::{bits::AsBytes, macros::Resource};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use crate::{
    error::AssetError, loader::AssetCacher, Asset, AssetDependency, AssetId, AssetInfo,
    AssetMetadata, AssetPath, Either, Settings,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Resource)]
pub struct AssetConfig {
    asset_path: PathBuf,
    cache_path: PathBuf,
}

impl AssetConfig {
    pub fn new(asset_path: PathBuf, cache_path: PathBuf) -> Self {
        AssetConfig {
            asset_path,
            cache_path,
        }
    }

    pub fn asset_path(&self) -> &PathBuf {
        &self.asset_path
    }

    pub fn cache_path(&self) -> &PathBuf {
        &self.cache_path
    }

    pub fn meta_path(&self, path: &Path) -> PathBuf {
        let mut meta_path = path.to_path_buf().into_os_string();
        meta_path.push(".meta");
        PathBuf::from(&meta_path)
    }

    pub fn cached_meta_path(&self, id: &AssetId) -> PathBuf {
        let mut cached_meta_path = self.cache_path.join(id.to_string());
        cached_meta_path.set_extension("meta");
        cached_meta_path
    }

    pub fn cached_asset_path(&self, id: &AssetId) -> PathBuf {
        self.cache_path.join(id.to_string())
    }

    pub fn asset_info_path(&self, path: &Path) -> PathBuf {
        let mut state = DefaultHasher::new();
        path.hash(&mut state);
        let hash = state.finish().to_string();
        let mut info_path = self.cache_path.join("lib");
        info_path.push(hash);
        info_path
    }

    pub fn load_metadata<S: Settings>(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<AssetMetadata<S>, AssetError> {
        let path = match path.into() {
            AssetPath::Id(id) => self.cached_meta_path(&id),
            AssetPath::Path(path) => self.meta_path(Path::new(&path)),
        };

        let metadata = std::fs::read_to_string(&path)?;
        toml::from_str::<AssetMetadata<S>>(&metadata).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse metadata: {}", e),
            )
            .into()
        })
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: impl Into<AssetPath>,
        metadata: &AssetMetadata<S>,
    ) -> Result<(), AssetError> {
        match path.into() {
            AssetPath::Id(id) => {
                let path = self.cached_meta_path(&id);
                let bytes = metadata.to_bytes();
                std::fs::write(&path, &bytes).map_err(|e| AssetError::from(e))?;
            }
            AssetPath::Path(path) => {
                let path = self.meta_path(Path::new(&path));
                let meta = toml::to_string(&metadata).map_err(|_| AssetError::InvalidSettings)?;

                std::fs::write(path, meta).map_err(|e| AssetError::from(e))?;
            }
        };

        Ok(())
    }

    pub fn load_asset_info<A: Asset>(&self, path: &Path) -> Result<AssetInfo, AssetError> {
        let path = self.asset_info_path(path);
        let bytes = std::fs::read(&path).map_err(|e| AssetError::from(e))?;
        AssetInfo::from_bytes(&bytes).ok_or(AssetError::InvalidData)
    }

    pub fn save_asset_info<A: Asset>(
        &self,
        path: &Path,
        info: &AssetInfo,
    ) -> Result<(), AssetError> {
        let bytes = info.to_bytes();
        std::fs::write(&path, &bytes).map_err(|e| AssetError::from(e))?;

        Ok(())
    }

    pub fn cache_asset<A: Asset>(
        &self,
        id: AssetId,
        asset: Either<(&A, &[AssetDependency], &AssetCacher<A>), &[u8]>,
    ) -> Result<(), AssetError> {
        let path = self.cached_asset_path(&id);
        match &asset {
            Either::Left((asset, dependencies, cacher)) => cacher.cache(&path, asset, dependencies),
            Either::Right(bytes) => std::fs::write(&path, &bytes).map_err(|e| AssetError::from(e)),
        }
    }
}
