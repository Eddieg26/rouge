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

fn main() {
    let mut world = world::World::new();
    world.register::<A>();
    world.register::<B>();
    let mut spawner = world.spawner();
    spawner.spawn().with(A).done();
    spawner.done();

    for (entity, a) in Query::<(Entity, &mut A), Not<B>>::new(&WorldCell::from(&world)) {
        println!("Entity: {:?}, A: {:?}", entity, a);
    }
    world.run(Root);
}
