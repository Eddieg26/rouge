use core::{component::Component, resource::Resource};
use event::Event;
use world::action::WorldAction;

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

pub struct A;
impl Component for A {}

pub struct ResA;
impl Resource for ResA {}

fn main() {
    let mut world = world::World::new();
    world.register::<A>();
    world.add_resource(ResA);
}
