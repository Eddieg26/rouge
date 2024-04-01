use super::{GraphNode, RenderPhase};
use crate::{
    core::{device::RenderQueue, surface::RenderSurfaceTexture},
    renderer::graph::{context::RenderContext, resources::GraphResources},
};
use rouge_ecs::{meta::{Access, AccessMeta, AccessType}, ResourceId};

pub struct PresentNode;

impl GraphNode for PresentNode {
    fn execute(&self, ctx: RenderContext) {
        ctx.system_arg::<&RenderQueue>().submit(ctx.collect());

        ctx.system_arg::<&mut RenderSurfaceTexture>()
            .present()
            .unwrap();
    }

    fn phase(&self) -> RenderPhase {
        RenderPhase::Finish
    }

    fn access(&self) -> Vec<AccessMeta> {
        vec![
            AccessMeta::new(AccessType::resource::<RenderQueue>(), Access::Read),
            AccessMeta::new(
                AccessType::id(ResourceId::from(GraphResources::SURFACE_ID)),
                Access::Write,
            ),
            AccessMeta::new(
                AccessType::resource::<RenderSurfaceTexture>(),
                Access::Write,
            ),
        ]
    }
}
