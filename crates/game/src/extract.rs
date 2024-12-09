use ecs::{
    system::{ArgItem, SystemArg},
    world::World,
};

pub trait Extractor: Send + Sync + 'static {
    type Input: SystemArg;
    type Output;

    fn register(_: &mut World) {}
    fn extract(input: &mut ArgItem<Self::Input>) -> Self::Output;
}
