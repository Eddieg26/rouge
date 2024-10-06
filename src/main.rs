use core::{component::Component, entity::Entity, resource::Resource};
use event::Event;
use system::systems::Root;
use world::{
    action::WorldAction,
    cell::WorldCell,
    query::{Not, Query},
    spawner::{SpawnedEntity, Spawner},
};

pub mod archetype;
pub mod core;
pub mod event;
pub mod system;
pub mod task;
pub mod world;

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

fn main() {
    let mut world = world::World::new();
    world.register::<A>();
    world.register::<B>();
    // let entity = world.spawn();
    // world.add_component(entity, A);
    // world.add_component(entity, B);
    let mut spawner = world.spawner();
    spawner.spawn().with(A).done();
    spawner.done();
    
    for (entity, a) in Query::<(Entity, &mut A), Not<B>>::new(&WorldCell::from(&world)) {
        println!("Entity: {:?}, A: {:?}", entity, a);
    }
    world.run(Root);
}
