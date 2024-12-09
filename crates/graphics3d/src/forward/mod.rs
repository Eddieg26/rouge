use crate::camera::{Camera, ClearFlag};
use asset::{AssetId, AssetRef};
use ecs::{
    core::{entity::Entity, resource::Resource},
    system::unlifetime::ReadRes,
};
use graphics::{
    renderer::{RenderContext, RenderGraphNode, RenderPass},
    resource::{BindGroupLayout, BlendMode, MaterialModel, Mesh, ShaderModel},
    Draw, RenderAssets, RenderDevice, RenderResourceExtractor, RenderState, RenderView,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardSubpass {
    Opaque = 0,
    Transparent = 1,
}

pub struct ForwardPass {
    pass: RenderPass,
    subpasses: [Subpass; 2],
}

impl ForwardPass {
    pub fn new() -> Self {
        Self {
            pass: RenderPass::new(),
            subpasses: [Subpass::new(), Subpass::new()],
        }
    }

    pub fn add_node(&mut self, pass: ForwardSubpass, node: impl DrawNode) {
        self.subpasses[pass as usize].nodes.push(Box::new(node));
    }

    pub fn pass(&self) -> &RenderPass {
        &self.pass
    }
}

impl RenderGraphNode for ForwardPass {
    fn name(&self) -> &str {
        "Forward"
    }

    fn run(&mut self, ctx: &mut RenderContext) {
        let mut encoder = ctx.encoder();
        let cameras = ctx.resource::<RenderAssets<RenderView<Camera>>>();

        for camera in cameras.values() {
            let clear = match camera.clear {
                Some(ClearFlag::Color(color)) => Some(color),
                _ => None,
            };

            let target = match camera.target {
                Some(target) => ctx.override_target(target),
                None => None,
            };

            if let Some(mut pass) = self.pass.begin(&mut encoder, ctx, target, clear) {
                let mut state = RenderState::new(&mut pass);
                for pass in &mut self.subpasses {
                    pass.run(camera, &mut state);
                }
            }
        }

        ctx.submit(encoder);
    }
}

pub struct Subpass {
    nodes: Vec<Box<dyn DrawNode>>,
}

impl Subpass {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn run(&mut self, camera: &RenderView<Camera>, state: &mut RenderState) {
        for node in &mut self.nodes {
            // Draw node
        }
    }
}

pub trait DrawNode: Send + Sync + 'static {}

pub struct ForwardLighting {
    layout: BindGroupLayout,
}

impl MaterialModel for ForwardLighting {
    fn model() -> ShaderModel {
        ShaderModel::Lit
    }

    fn bind_group_layout(&self) -> Option<&BindGroupLayout> {
        Some(&self.layout)
    }
}

impl RenderResourceExtractor for ForwardLighting {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(device: ecs::system::ArgItem<Self::Arg>) -> Result<Self, graphics::ExtractError> {
        let layout = BindGroupLayout::create(&device, &[]);

        Ok(Self { layout })
    }
}

impl Resource for ForwardLighting {}

pub struct DrawMesh {
    pub entity: Entity,
    pub mesh: AssetId,
    pub material: AssetId,
}

impl Draw for DrawMesh {
    fn entity(&self) -> Entity {
        self.entity
    }
}

pub struct DrawMeshNode {
    mode: BlendMode,
}

impl DrawNode for DrawMeshNode {}
