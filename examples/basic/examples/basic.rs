use asset::{
    asset::{Asset, AssetId, AssetRef, Assets},
    database::{events::AssetEvent, AssetDatabase},
    embed_asset,
    importer::{DefaultProcessor, ImportError, Importer},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache},
        embedded::EmbeddedAssets,
        local::LocalAssets,
        source::{AssetPath, AssetSource},
        vfs::VirtualFs,
        AssetIo, AssetIoError,
    },
    plugin::{AssetExt, AssetPlugin},
    AsyncReadExt, AsyncWriteExt,
};
use std::{future::Future, path::PathBuf};
// use asset::{
//     asset::{Asset, AssetId, AssetType},
//     io::{embed::EmbeddedFS, local::LocalFS, AssetSourceConfig},
// };
use ecs::{
    core::{
        component::Component,
        entity::Entity,
        resource::{Res, Resource},
    },
    event::{Event, Events},
    system::systems::Root,
    world::{
        self,
        action::WorldAction,
        cell::WorldCell,
        query::{Not, Query},
        World,
    },
};
use game::{Game, PostInit};
use pollster::block_on;

pub struct TestEvent;
impl Event for TestEvent {}

pub struct TestAction;

impl WorldAction for TestAction {
    fn execute(self, _: &mut world::World) -> Option<()> {
        println!("Test Action!");
        Some(())
    }
}

#[derive(Debug)]
pub struct A;
impl Component for A {}

pub struct B;
impl Component for B {}

pub struct ResA;
impl Resource for ResA {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PlainText(String);
impl Asset for PlainText {}

impl Importer for PlainText {
    type Asset = PlainText;
    type Settings = ();
    type Processor = DefaultProcessor<Self, Self::Settings>;
    type Error = AssetIoError;

    fn import<'a>(
        _ctx: &'a mut asset::importer::ImportContext<Self::Asset, Self::Settings>,
        reader: &'a mut dyn asset::io::AssetReader,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + 'a {
        async move {
            let mut data = String::new();
            reader.read_to_string(&mut data).await?;
            Ok(PlainText(data))
        }
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

const ID: uuid::Uuid = uuid::Uuid::from_u128(0);

fn main() {
    let fs = VirtualFs::new();
    let mut writer = block_on(fs.writer("test.txt".as_ref())).unwrap();
    let _ = block_on(writer.write_all(b"This is test text"));
    std::mem::drop(writer);

    block_on(fs.create_dir("assets".as_ref())).unwrap();

    let mut writer = block_on(fs.writer("assets/test.txt".as_ref())).unwrap();
    let _ = block_on(writer.write_all(b"This is test text"));
    std::mem::drop(writer);

    let mut reader = block_on(fs.reader("test.txt".as_ref())).unwrap();
    let mut data = String::new();
    let _ = block_on(reader.read_to_string(&mut data));

    println!("Data: {:?}", data);

    block_on(fs.remove_dir("assets".as_ref())).unwrap();

    println!("{}", fs);

    let embedded = EmbeddedAssets::new();
    let id = AssetRef::<PlainText>::from(ID);
    embed_asset!(embedded, id, "embedded.txt", ());

    // let embedded = EmbeddedAssets::new();
    // let bytes = include_bytes!("embedded.txt");
    // embedded.embed("embedded.txt", bytes);

    // Game::new()
    //     .add_plugin(AssetPlugin)
    //     .register_asset::<PlainText>()
    //     .add_importer::<PlainText>()
    //     .embed_assets("basic", "", embedded)
    //     .add_systems(PostInit, |db: Res<AssetDatabase>| {
    //         db.load(["test.txt"]);
    //     })
    //     .observe::<AssetEvent<PlainText>, _>(
    //         |events: Res<Events<AssetEvent<PlainText>>>,
    //          db: Res<AssetDatabase>,
    //          assets: Res<Assets<PlainText>>| {
    //             for event in events.iter() {
    //                 match event {
    //                     AssetEvent::Loaded(id) => {
    //                         println!("Loaded: {:?} ", id);
    //                         if let Some(text) = assets.get(id) {
    //                             println!("Text: {:?}", text.0);
    //                         }
    //                     }
    //                     AssetEvent::Unloaded { id, .. } => {
    //                         println!("Unloaded: {:?}", id);
    //                     }
    //                     AssetEvent::Failed { id, error } => {
    //                         println!("Failed: {:?} {:?}", id, error);
    //                     }
    //                     AssetEvent::Imported(id) => {
    //                         let library = db.library();
    //                         let library = library.read_arc_blocking();
    //                         let path = library.get_path(id);
    //                         println!("Imported Path: {:?}", path);
    //                     }
    //                     AssetEvent::DepsLoaded(id) => {
    //                         println!("DepsLoaded: {:?}", id);
    //                     }
    //                 }
    //             }
    //         },
    //     )
    //     .observe::<ImportError, _>(|errors: Res<Events<ImportError>>| {
    //         for error in errors.iter() {
    //             println!("Import Error: {:?}", error);
    //         }
    //     })
    //     .set_runner(runner)
    //     .run();
}

fn runner(mut game: Game) {
    game.startup();
    loop {
        game.update();
    }
}
