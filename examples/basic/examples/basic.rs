use std::future::Future;

use asset::{
    asset::{Asset, AssetId},
    database::{events::AssetEvent, AssetDatabase},
    importer::{DefaultProcessor, ImportError, Importer},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache},
        embedded::EmbeddedFs,
        local::LocalAssets,
        source::{AssetPath, AssetSource},
        AssetIoError,
    },
    plugin::{AssetExt, AssetPlugin},
};
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
use futures_lite::{AsyncReadExt, StreamExt};
use game::Game;
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

fn main() {
    Game::new()
        .add_plugin(AssetPlugin)
        .register_asset::<PlainText>()
        .add_importer::<PlainText>()
        .observe::<AssetEvent<PlainText>, _>(
            |events: Res<Events<AssetEvent<PlainText>>>, db: Res<AssetDatabase>| {
                for event in events.iter() {
                    match event {
                        AssetEvent::Loaded(id) => {
                            println!("Loaded: {:?} ", id);
                        }
                        AssetEvent::Unloaded { id, .. } => {
                            println!("Unloaded: {:?}", id);
                        }
                        AssetEvent::Failed { id, error } => {
                            println!("Failed: {:?} {:?}", id, error);
                        }
                        AssetEvent::Imported(id) => {
                            let library = db.library();
                            let library = library.read_arc_blocking();
                            let path = library.get_path(id);
                            println!("Imported Path: {:?}", path);
                        }
                        AssetEvent::DepsLoaded(id) => {
                            println!("DepsLoaded: {:?}", id);
                        }
                    }
                }
            },
        )
        .observe::<ImportError, _>(|errors: Res<Events<ImportError>>| {
            for error in errors.iter() {
                println!("Import Error: {:?}", error);
            }
        })
        .set_runner(runner)
        .run();
}

fn runner(mut game: Game) {
    game.startup();
    loop {
        game.update();
    }
}
