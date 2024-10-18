use asset::io::source::AssetPath;
// use asset::{
//     asset::{Asset, AssetId, AssetType},
//     io::{embed::EmbeddedFS, local::LocalFS, AssetSourceConfig},
// };
use ecs::{
    core::{component::Component, entity::Entity, resource::Resource},
    event::Event,
    system::systems::Root,
    world::{
        self,
        action::WorldAction,
        cell::WorldCell,
        query::{Not, Query},
    },
};

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

#[derive(serde::Serialize, serde::Deserialize)]
struct TestAsset;
// impl Asset for TestAsset {}

fn main() {
    let path = "remote://assets/texture.png@main";
    let asset_path = AssetPath::from_str(path);
    println!("{:?}", asset_path.path());
}
