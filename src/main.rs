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

fn main() {
    let mut world = world::World::new();
    world.register_event::<TestEvent>();
    world.actions().add(TestAction);
    world.run(Root);
}
