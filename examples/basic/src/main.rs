use rouge_asset::{
    database::{AssetInfo, AssetLibrary},
    filesystem::{FileSystem, PathExt},
    AssetId,
};
use rouge_game::game::Game;
use rouge_winit::WinitPlugin;
use std::path::Path;

fn main() {
    // Game::new().add_plugin(WinitPlugin::new()).run();

    let filesystem = FileSystem::new("assets");
    // let exists = filesystem.exists(&Path::new("test.txt"));

    let mut library = AssetLibrary::new();
    let res = filesystem.list_recursive(&Path::new("")).unwrap();
    for r in res {
        let id = AssetId::new();
        let path = r.as_path().normalize();
        library.insert(id, AssetInfo::new::<()>(path.to_str().unwrap()));
    }

    let library = toml::to_string(&library).unwrap();
    filesystem
        .write_str(&Path::new(".meta/assets.lib"), &library)
        .unwrap()

    // println!("File exists: {}", exists);
}
