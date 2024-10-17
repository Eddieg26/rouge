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
    // let mut world = world::World::new();
    // world.register::<A>();
    // world.register::<B>();
    // let mut spawner = world.spawner();
    // spawner.spawn().with(A).done();
    // spawner.done();

    // for (entity, a) in Query::<(Entity, &mut A), Not<B>>::new(&WorldCell::from(&world)) {
    //     println!("Entity: {:?}, A: {:?}", entity, a);
    // }
    // world.run(Root);

    // let mut embedded_fs = EmbeddedFS::new();
    // let bytes = include_bytes!("test.txt");
    // embedded_fs.embed("test.txt", bytes);

    // let mut reader = embedded_fs.reader();
    // let mut buffer = Vec::<u8>::new();
    // pollster::block_on(reader.read_to_end("test.txt".as_ref(), buffer.as_mut())).unwrap();

    // let text = std::str::from_utf8(&buffer).unwrap();
    // println!("{}", text);

    // let mut writer = embedded_fs.writer();
    // let _ =
    //     pollster::block_on(writer.write("test.txt".as_ref(), "This is different text".as_bytes()));

    // let mut reader = embedded_fs.reader();
    // let mut buffer = Vec::<u8>::new();
    // pollster::block_on(reader.read_to_end("test.txt".as_ref(), buffer.as_mut())).unwrap();

    // let text = std::str::from_utf8(&buffer).unwrap();
    // println!("{}", text);
}
