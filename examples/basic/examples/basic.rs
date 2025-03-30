use std::ops::Range;

use asset::{derive::Asset, embed_asset, embedded::EmbeddedFs, AssetExt, AssetRef};
use ecs::{
    derive::Component,
    system::unlifetime::{Read, WriteRes},
    world::{action::WorldActions, builtin::actions::Spawn},
    Entity,
};
use game::{Game, Init};
use render::{
    derive::{AsBinding, ShaderType},
    wgpu::{RenderPassColorAttachment, RenderPassDescriptor},
    BatchKey, Color, Draw, DrawCall, DrawFunctions, DrawId, DrawPass, DrawPhase, ExtractedView,
    IntoDrawCall, Material, MaterialRef, Mesh, MeshAttribute, MeshAttributeType,
    MeshAttributeValues, MeshTopology, PassBuilder, RenderAppExt, RenderContext, RenderGraphPass,
    RenderMesh, RenderPlugin, RenderState, RenderTarget, RenderView, ShaderPath, ShaderSource,
    SubGraph, ViewBuffer, ViewDrawCalls, ViewPass,
};
use spatial::Transform;
use uuid::Uuid;

const VERTEX_SHADER_ID: Uuid = Uuid::from_u128(0);
const FRAGMENT_SHADER_ID: Uuid = Uuid::from_u128(1);
const MATERIAL_ID: Uuid = Uuid::from_u128(0);
const MESH_ID: Uuid = Uuid::from_u128(1);

fn main() {
    const QUAD: &[glam::Vec2] = &[
        glam::Vec2::new(-1.0, -1.0), // Bottom-left
        glam::Vec2::new(1.0, -1.0),  // Bottom-right
        glam::Vec2::new(-1.0, 1.0),  // Top-left
        glam::Vec2::new(1.0, -1.0),  // Bottom-right
        glam::Vec2::new(1.0, 1.0),   // Top-right
        glam::Vec2::new(-1.0, 1.0),  // Top-left
    ];

    let quad = Mesh::new(MeshTopology::TriangleList).with_attribute(MeshAttribute::new(
        MeshAttributeType::Position,
        MeshAttributeValues::Vec2(QUAD.to_vec()),
    ));

    let embedded = EmbeddedFs::new("assets");

    let vs_id = AssetRef::<ShaderSource>::from(VERTEX_SHADER_ID);
    embed_asset!(embedded, vs_id, "assets/vertex.wgsl", ());

    let fs_id = AssetRef::<ShaderSource>::from(FRAGMENT_SHADER_ID);
    embed_asset!(embedded, fs_id, "assets/fragment.wgsl", ());

    Game::new()
        .add_plugin(RenderPlugin)
        .add_draw::<DrawMesh2d<UnlitColor>, Opaque2d>()
        .embed_assets("embedded", embedded)
        .register::<Camera2d>()
        .register::<Mesh2d>()
        .add_asset(MATERIAL_ID, UnlitColor::new(Color::red()), vec![])
        .add_asset(MESH_ID, quad, vec![])
        .add_systems(Init, |actions: WorldActions| {
            actions.add(Spawn::new().with(Camera2d { depth: 0 }));
            actions.add(
                Spawn::new()
                    .with(Mesh2d::new(MESH_ID.into()))
                    .with(MaterialRef::<UnlitColor>::new(MATERIAL_ID)),
            );
        })
        .run();
}

#[derive(AsBinding, Asset, Clone, serde::Serialize, serde::Deserialize)]
#[uniform(0)]
pub struct UnlitColor {
    #[uniform]
    pub color: Color,
}

impl UnlitColor {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl Material for UnlitColor {
    fn mode() -> render::BlendMode {
        render::BlendMode::Opaque
    }

    fn shader() -> impl Into<render::ShaderPath> {
        ShaderPath::from(FRAGMENT_SHADER_ID)
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct Mesh2dData {
    model: glam::Mat4,
}

impl render::MeshData for Mesh2dData {
    fn world(&self) -> glam::Mat4 {
        self.model
    }
}

#[derive(Default, Component, Clone, Copy)]
pub struct Camera2d {
    depth: i32,
}

#[derive(ShaderType, Clone, Copy)]
pub struct Camera2dView {
    pub world: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

impl render::View for Camera2dView {
    type Query = (Entity, Read<Camera2d>, Read<Transform>);

    fn extract<'a>(
        query: <Self::Query as ecs::prelude::query::BaseQuery>::Item<'a>,
    ) -> ExtractedView<Self> {
        let (entity, camera, transform) = query;
        ExtractedView {
            entity,
            depth: camera.depth,
            view: Camera2dView {
                world: transform.world(),
                view: transform.world().inverse(),
                projection: glam::Mat4::IDENTITY,
            },
        }
    }
}

pub struct Opaque2d {
    z_order: i32,
}

impl DrawPhase for Opaque2d {
    type View = Camera2dView;
}

pub struct Transparent2d {
    z_order: i32,
}

impl DrawPhase for Transparent2d {
    type View = Camera2dView;
}

#[derive(Component, Clone, Copy)]
pub struct Mesh2d(AssetRef<Mesh>);
impl Mesh2d {
    pub fn new(mesh: AssetRef<Mesh>) -> Self {
        Self(mesh)
    }
}

impl std::ops::Deref for Mesh2d {
    type Target = AssetRef<Mesh>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<AssetRef<Mesh>> for Mesh2d {
    fn as_ref(&self) -> &AssetRef<Mesh> {
        &self.0
    }
}

#[derive(Clone, Copy)]
pub struct DrawMesh2d<M: Material> {
    enitity: Entity,
    material: AssetRef<M>,
    mesh: AssetRef<render::Mesh>,
    data: Mesh2dData,
    z_order: i32,
}

impl<M: Material> Draw for DrawMesh2d<M> {
    type View = Camera2dView;

    type Mesh = Mesh2dData;

    type Material = M;

    type Query = (Entity, Read<MaterialRef<Self::Material>>, Read<Mesh2d>);

    fn entity(&self) -> Entity {
        self.enitity
    }

    fn data(&self) -> Self::Mesh {
        self.data
    }

    fn material(&self) -> AssetRef<Self::Material> {
        self.material
    }

    fn mesh(&self) -> AssetRef<render::Mesh> {
        self.mesh
    }

    fn shader() -> impl Into<ShaderPath> {
        ShaderPath::from(VERTEX_SHADER_ID)
    }

    fn vertex_layout() -> &'static [render::wgpu::VertexFormat] {
        &[render::wgpu::VertexFormat::Float32x2]
    }

    fn instance_layout() -> &'static [render::wgpu::VertexFormat] {
        &[
            render::wgpu::VertexFormat::Float32x4,
            render::wgpu::VertexFormat::Float32x4,
            render::wgpu::VertexFormat::Float32x4,
            render::wgpu::VertexFormat::Float32x4,
        ]
    }

    fn extract<'a>(query: <Self::Query as ecs::prelude::query::BaseQuery>::Item<'a>) -> Self {
        let (entity, material, mesh) = query;
        Self {
            enitity: entity,
            material: (*material).into(),
            mesh: mesh.as_ref().clone(),
            data: Mesh2dData {
                model: glam::Mat4::IDENTITY,
            },
            z_order: 0,
        }
    }
}

impl<M: Material> IntoDrawCall<Opaque2d> for DrawMesh2d<M> {
    fn into_draw_call(&self, view: &RenderView<<Opaque2d as DrawPhase>::View>) -> Opaque2d {
        Opaque2d {
            z_order: self.z_order,
        }
    }
}

impl<M: Material> IntoDrawCall<Transparent2d> for DrawMesh2d<M> {
    fn into_draw_call(
        &self,
        view: &RenderView<<Transparent2d as DrawPhase>::View>,
    ) -> Transparent2d {
        Transparent2d {
            z_order: self.z_order,
        }
    }
}

pub struct Render2d;

impl SubGraph for Render2d {
    const NAME: render::Name = "Render2d";
}

pub struct Main2dRenderPass;

impl ViewPass for Main2dRenderPass {
    type View = Camera2dView;

    const NAME: render::Name = "Main2dRenderPass";

    fn setup(
        builder: &mut PassBuilder,
    ) -> impl Fn(&mut RenderContext, &RenderView<Self::View>) + 'static {
        let target = builder.write::<RenderTarget>();

        move |ctx, view| {
            let draw_functions = ctx.world().resource::<DrawFunctions<Self::View>>();
            let meshes = ctx.world().resource::<render::RenderAssets<RenderMesh>>();
            let view_buffer = ctx.world().resource::<ViewBuffer<Self::View>>();

            let mut encoder = ctx.encoder();
            // let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            //     label: Some(Self::NAME),
            //     color_attachments: &[Some(RenderPassColorAttachment {
            //         view: (),
            //         resolve_target: (),
            //         ops: (),
            //     })],
            //     depth_stencil_attachment: None,
            //     timestamp_writes: None,
            //     occlusion_query_set: None,
            // });

            // let mut state = RenderState::new(&mut pass);

            // let opaque = ctx.world().resource::<ViewDrawCalls<Opaque2d>>();
            // opaque.draw(ctx, view, meshes, view_buffer, draw_functions, &mut state);
        }
    }
}

// fn main() {
// let embedded = EmbeddedFs::new("assets");
// let vs_id = AssetRef::<ShaderSource>::from(VERTEX_SHADER_ID);
// embed_asset!(embedded, vs_id, "assets/vertex.wgsl", ());
// let fs_id = AssetRef::<ShaderSource>::from(FRAGMENT_SHADER_ID);
// embed_asset!(embedded, fs_id, "assets/fragment.wgsl", ());

// let triangle =
//     Mesh::new(MeshTopology::TriangleList).with_attribute(MeshAttribute::Position(vec![
//         Vec3::new(0.0, 0.5, 0.0),
//         Vec3::new(-0.5, -0.5, 0.0),
//         Vec3::new(0.5, -0.5, 0.0),
//     ]));
// let mesh_id = AssetId::from::<Mesh>(MESH_ID);

// Game::new()
//     .add_plugin(RenderPlugin)
//     .add_asset(mesh_id, triangle, vec![])
//     .scoped_resource::<RenderGraphBuilder>(|_, builder| {
//         builder.add_node(BasicRenderNode::new());
//     })
//     .scoped_sub_app::<RenderApp>(|_, app| {
//         app.observe::<ExtractError, _>(|errors: Res<Events<ExtractError>>| {
//             for error in errors.iter() {
//                 println!("Extract Error: {:?}", error);
//             }
//         });
//     })
//     .embed_assets("basic", embedded)
//     .run();
// }

// pub struct BasicRenderNode {
//     pass: RenderPass,
// }

// impl BasicRenderNode {
//     pub fn new() -> Self {
//         Self {
//             pass: RenderPass::new().with_color(
//                 Attachment::Surface,
//                 None,
//                 StoreOp::Store,
//                 Some(Color::blue()),
//             ),
//         }
//     }
// }

// impl RenderGraphNode for BasicRenderNode {
//     fn name(&self) -> &str {
//         "Basic"
//     }

//     fn run(&mut self, ctx: &mut graphics::renderer::context::RenderContext) {
//         let mut encoder = ctx.encoder();
//         if let Some(mut pass) = self.pass.begin(&mut encoder, ctx, None, None) {
//             let mut state = RenderState::new(&mut pass);
//             let meshes = ctx.resource::<RenderAssets<RenderMesh>>();
//             let calls = ctx.resource::<DrawCalls<DrawMesh>>();

//             for call in calls {
//                 let mesh = match meshes.get(&call.mesh) {
//                     Some(mesh) => mesh,
//                     None => continue,
//                 };

//                 let position = match mesh.vertex_buffer(MeshAttributeKind::Position) {
//                     Some(position) => position,
//                     None => continue,
//                 };

//                 state.set_vertex_buffer(0, position.slice(..));

//             }
//         }

//         ctx.submit(encoder);
//     }
// }

// Draw Material
// Set Render Pipeline
// Set View Bind Group
// Set Mesh Bind Group
// Set Material Bind Group
// Set Extra Bind Group
// Draw Mesh
