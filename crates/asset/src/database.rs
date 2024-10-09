use crate::{asset::AssetPath, config::AssetConfig, library::AssetLibrary};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub enum AssetAction {
    Load { paths: Vec<AssetPath> },
    Import { paths: Vec<PathBuf> },
}

pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    library: Arc<RwLock<AssetLibrary>>,
}
