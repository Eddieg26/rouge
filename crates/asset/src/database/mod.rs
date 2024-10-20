use std::sync::Arc;
use config::AssetConfig;

pub mod config;


pub struct AssetDatabase {
    config: Arc<AssetConfig>,
}