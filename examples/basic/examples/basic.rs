use std::num::NonZeroU64;

use asset::{
    embed_asset,
    io::{cache::LoadPath, embedded::EmbeddedFs},
    plugin::AssetExt,
    Asset, AssetId, AssetRef,
};
use ecs::{
    core::{
        component::Component,
        entity::Entity,
        resource::{Res, Resource},
    },
    event::Events,
    system::unlifetime::{Read, ReadRes, StaticQuery},
    world::World,
};
use game::{Game, Main, SMain};
use glam::Vec3;
use graphics::{
    core::{Color, ExtractError},
    encase::ShaderType,
    plugin::{RenderApp, RenderAppExt, RenderPlugin},
    renderer::{
        graph::{RenderGraphBuilder, RenderGraphNode},
        pass::{Attachment, RenderPass, StoreOp},
    },
    resource::{
        globals::GlobalView, plugin::MaterialAppExt, BindGroupLayout, BindGroupLayoutBuilder,
        BlendMode, Material, MaterialBinding, Mesh, MeshAttribute, MeshAttributeKind, MeshPipeline,
        MeshTopology, RenderMesh, ShaderSource, UniformBuffer, Unlit, VertexAttribute,
    },
    wgpu::{PrimitiveState, ShaderStages},
    CreateBindGroup, Draw, DrawCalls, DrawExtractor, RenderAssets, RenderDevice,
    RenderResourceExtractor, RenderState,
};
use uuid::Uuid;

const VERTEX_SHADER_ID: Uuid = Uuid::from_u128(0);
const FRAGMENT_SHADER_ID: Uuid = Uuid::from_u128(1);
const MATERIAL_ID: Uuid = Uuid::from_u128(0);
const MESH_ID: Uuid = Uuid::from_u128(1);

fn main() {
    let embedded = EmbeddedFs::new("assets");
    let vs_id = AssetRef::<ShaderSource>::from(VERTEX_SHADER_ID);
    embed_asset!(embedded, vs_id, "assets/vertex.wgsl", ());
    let fs_id = AssetRef::<ShaderSource>::from(FRAGMENT_SHADER_ID);
    embed_asset!(embedded, fs_id, "assets/fragment.wgsl", ());
    let mat_id = AssetId::from::<UnlitColor>(MATERIAL_ID);

    let triangle =
        Mesh::new(MeshTopology::TriangleList).with_attribute(MeshAttribute::Position(vec![
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
        ]));
    let mesh_id = AssetId::from::<Mesh>(MESH_ID);

    Game::new()
        .add_plugin(RenderPlugin)
        .add_material::<UnlitColor>()
        .add_asset(mat_id, UnlitColor::from(Color::green()), vec![])
        .add_asset(mesh_id, triangle, vec![])
        .scoped_resource::<RenderGraphBuilder>(|_, builder| {
            builder.add_node(BasicRenderNode::new());
        })
        .scoped_sub_app::<RenderApp>(|_, app| {
            app.observe::<ExtractError, _>(|errors: Res<Events<ExtractError>>| {
                for error in errors.iter() {
                    println!("Extract Error: {:?}", error);
                }
            });
        })
        .add_draw_call_extractor::<DrawMesh>()
        .embed_assets("basic", embedded)
        .run();
}

pub struct GlobalValue {
    layout: BindGroupLayout,
    buffer: UniformBuffer<f32>,
}
impl GlobalValue {
    pub fn new(device: &RenderDevice) -> Self {
        let layout = BindGroupLayoutBuilder::new()
            .with_uniform_buffer(0, ShaderStages::VERTEX, true, NonZeroU64::new(4), None)
            .build(device);
        let buffer = UniformBuffer::new(device, 1f32);

        Self { layout, buffer }
    }

    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    pub fn buffer(&self) -> &UniformBuffer<f32> {
        &self.buffer
    }
}

impl MaterialBinding for GlobalValue {
    fn bind_group_layout(&self) -> &BindGroupLayout {
        todo!()
    }
}

impl Resource for GlobalValue {}
impl RenderResourceExtractor for GlobalValue {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self, ExtractError> {
        Ok(Self::new(&arg))
    }
}

pub struct BasicMeshPipeline {
    layout: BindGroupLayout,
}

impl MeshPipeline for BasicMeshPipeline {
    type View = GlobalValue;
    type Mesh = BasicMeshPipeline;

    fn primitive() -> PrimitiveState {
        PrimitiveState::default()
    }

    fn attributes() -> Vec<VertexAttribute> {
        vec![VertexAttribute::Vec3]
    }

    fn shader() -> impl Into<LoadPath> {
        LoadPath::Id(AssetId::from::<ShaderSource>(VERTEX_SHADER_ID))
    }
}

impl MaterialBinding for BasicMeshPipeline {
    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

impl RenderResourceExtractor for BasicMeshPipeline {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self, ExtractError> {
        let device = arg;
        let layout = BindGroupLayoutBuilder::new()
            .with_uniform_buffer(
                0,
                ShaderStages::VERTEX,
                true,
                Some(glam::Mat4::min_size()),
                None,
            )
            .build(&device);

        Ok(Self { layout })
    }
}

impl Resource for BasicMeshPipeline {}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
pub struct UnlitColor {
    #[uniform(0)]
    pub color: Color,
}

impl From<Color> for UnlitColor {
    fn from(color: Color) -> Self {
        Self { color }
    }
}

impl Material for UnlitColor {
    type Pipeline = BasicMeshPipeline;
    type Model = Unlit;

    fn mode() -> BlendMode {
        BlendMode::Opaque
    }

    fn shader() -> impl Into<LoadPath> {
        LoadPath::Id(AssetId::from::<ShaderSource>(FRAGMENT_SHADER_ID))
    }
}

pub struct BasicRenderNode {
    pass: RenderPass,
}

impl BasicRenderNode {
    pub fn new() -> Self {
        Self {
            pass: RenderPass::new().with_color(
                Attachment::Surface,
                None,
                StoreOp::Store,
                Some(Color::blue()),
            ),
        }
    }
}

impl RenderGraphNode for BasicRenderNode {
    fn name(&self) -> &str {
        "Basic"
    }

    fn run(&mut self, ctx: &mut graphics::renderer::context::RenderContext) {
        let mut encoder = ctx.encoder();
        if let Some(mut pass) = self.pass.begin(&mut encoder, ctx, None, None) {
            let mut state = RenderState::new(&mut pass);
            let meshes = ctx.resource::<RenderAssets<RenderMesh>>();
            let calls = ctx.resource::<DrawCalls<DrawMesh>>();

            for call in calls {
                let mesh = match meshes.get(&call.mesh) {
                    Some(mesh) => mesh,
                    None => continue,
                };

                let position = match mesh.vertex_buffer(MeshAttributeKind::Position) {
                    Some(position) => position,
                    None => continue,
                };

                state.set_vertex_buffer(0, position.slice(..));

                // Set object bind group
            }
        }

        ctx.submit(encoder);
    }
}

pub struct DrawMesh {
    pub entity: Entity,
    pub mesh: AssetId,
    pub material: AssetRef<UnlitColor>,
}

impl Draw for DrawMesh {
    fn entity(&self) -> ecs::core::entity::Entity {
        self.entity
    }
}

impl DrawExtractor for DrawMesh {
    type Arg = SMain<StaticQuery<(Entity, Read<MeshRenderer>)>>;
    type Draw = DrawMesh;

    fn extract(calls: &mut graphics::DrawCalls<Self::Draw>, arg: ecs::system::ArgItem<Self::Arg>) {
        for (entity, renderer) in arg.into_inner() {
            calls.add(DrawMesh {
                entity,
                mesh: renderer.mesh,
                material: renderer.material,
            });
        }
    }
}

pub struct MeshRenderer {
    pub mesh: AssetId,
    pub material: AssetRef<UnlitColor>,
}

impl Component for MeshRenderer {}

pub struct DrawMaterial<M: Material> {
    pub material: AssetRef<M>,
}

// Draw Material
// Set Render Pipeline
// Set View Bind Group
// Set Mesh Bind Group
// Set Material Bind Group
// Set Extra Bind Group
// Draw Mesh