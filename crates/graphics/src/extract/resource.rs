use ecs::{
    core::{resource::Resource, IndexMap},
    system::{ArgItem, SystemArg},
    world::{
        action::{WorldAction, WorldActionFn},
        World,
    },
};
use std::any::TypeId;

use crate::RenderDevice;

pub trait RenderResource: Resource + Send + Sync {
    type Extract: SystemArg;

    fn can_extract(_: &World) -> bool {
        true
    }

    fn extract(device: &RenderDevice, arg: ArgItem<Self::Extract>) -> Self;
}

pub struct ExtractResource<R: RenderResource>(std::marker::PhantomData<R>);
impl<R: RenderResource> ExtractResource<R> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<R: RenderResource> WorldAction for ExtractResource<R> {
    fn execute(self, world: &mut World) -> Option<()> {
        let arg = R::Extract::get(unsafe { world.cell() });
        let device = world.resource::<RenderDevice>();
        let resource = R::extract(device, arg);
        world.add_resource(resource);
        Some(())
    }
}

pub struct ResourceExtractors(IndexMap<TypeId, WorldActionFn>);
impl ResourceExtractors {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn add<R: RenderResource>(&mut self) {
        self.0.insert(
            TypeId::of::<R>(),
            WorldActionFn::from(ExtractResource::<R>::new()),
        );
    }
}

impl Resource for ResourceExtractors {}
