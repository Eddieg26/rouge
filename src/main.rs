use core::component::Component;
use event::Event;
use system::systems::Root;
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
    fn execute(self, _: &mut world::World) {
        println!("Test Action!");
    }
}

pub struct A;
impl Component for A {}

fn main() {
    let mut world = world::World::new();
    world.register::<A>();
    world.register_event::<TestEvent>();
    world.actions().add(TestAction);
    world.run(Root);
}
