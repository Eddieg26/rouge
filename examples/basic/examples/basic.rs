use asset::{
    asset::{Asset, AssetId, AssetRef, Assets},
    database::{events::AssetEvent, AssetDatabase},
    embed_asset,
    importer::{DefaultProcessor, ImportContext, ImportError, Importer},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache},
        embedded::EmbeddedFs,
        local::LocalFs,
        source::{AssetPath, AssetSource},
        vfs::VirtualFs,
        AssetIoError, FileSystem,
    },
    plugin::{AssetExt, AssetPlugin},
    AsyncReadExt, AsyncWriteExt,
};
use graphics::plugin::RenderPlugin;
use std::{future::Future, path::PathBuf};
use window::plugin::WindowPlugin;
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
use game::{Game, PostInit, Update};
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

    async fn import(
        _ctx: &mut ImportContext<'_, Self::Asset, Self::Settings>,
        reader: &mut dyn asset::io::AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        let mut data = String::new();
        reader.read_to_string(&mut data).await?;
        Ok(PlainText(data))
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

const ID: uuid::Uuid = uuid::Uuid::from_u128(0);

fn main() {
    let embedded = EmbeddedFs::new("assets");
    let id = AssetRef::<PlainText>::from(ID);
    let _ = embed_asset!(embedded, id, "assets/embedded.txt", ());

    Game::new()
        .add_plugin(RenderPlugin)
        .register_asset::<PlainText>()
        .add_importer::<PlainText>()
        .embed_assets("basic", embedded)
        .add_systems(PostInit, |db: Res<AssetDatabase>| {
            db.load(["basic://assets/embedded.txt"]);
        })
        .observe::<AssetEvent<PlainText>, _>(|events: Res<Events<AssetEvent<PlainText>>>| {
            for event in events.iter() {
                match event {
                    AssetEvent::Imported { id } => println!("Imported: {:?}", id),
                    AssetEvent::Added { id } => println!("Loaded: {:?}", id),
                    _ => (),
                }
            }
        })
        .observe::<ImportError, _>(|errors: Res<Events<ImportError>>| {
            for error in errors.iter() {
                println!("Import Error: {:?}", error);
            }
        })
        .run();
}
