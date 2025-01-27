use crate::RenderDevice;
use ecs::{
    core::{resource::Resource, IndexMap},
    system::{ArgItem, SystemArg},
    world::{cell::WorldCell, World},
};
use std::any::TypeId;

pub trait RenderResourceExtractor: Resource + Sized + Send {
    type Arg: SystemArg;

    fn can_extract(world: &World) -> bool {
        world.has_resource::<RenderDevice>() && Self::Arg::validate(&unsafe { world.cell() })
    }

    fn extract(device: &RenderDevice, arg: ArgItem<Self::Arg>) -> Self;
}

pub struct ErasedResourceExtractor {
    extract: fn(&RenderDevice, WorldCell) -> bool,
}

impl ErasedResourceExtractor {
    pub fn new<R: RenderResourceExtractor>() -> Self {
        Self {
            extract: |device, world| {
                if R::can_extract(world.get()) {
                    let resource = R::extract(device, R::Arg::get(world));
                    world.get_mut().add_resource(resource);
                    false
                } else {
                    true
                }
            },
        }
    }
}

#[derive(Default)]
pub struct RenderResourceExtractors {
    extractors: IndexMap<TypeId, ErasedResourceExtractor>,
}

impl RenderResourceExtractors {
    pub fn add<R: RenderResourceExtractor>(&mut self) {
        self.extractors
            .insert(TypeId::of::<R>(), ErasedResourceExtractor::new::<R>());
    }

    pub(crate) fn extract(&mut self, world: &World) {
        let device = world.resource::<RenderDevice>();
        let world = unsafe { world.cell() };

        self.extractors
            .retain(|_, extractor| (extractor.extract)(device, world));
    }
}

impl Resource for RenderResourceExtractors {}
