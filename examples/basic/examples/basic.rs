use asset::{derive::Asset, embed_asset, embedded::EmbeddedFs, AssetExt, AssetRef};
use ecs::{
    derive::Component,
    system::unlifetime::Read,
    world::{action::WorldActions, builtin::actions::Spawn},
    Entity,
};
use game::{Game, Init};
use render::{
    derive::{AsBinding, ShaderType},
    Color, Draw, DrawPass, MainDrawPass, Material, Mesh, MeshAttribute, MeshAttributeType,
    MeshAttributeValues, MeshTopology, Operations, RenderAppExt, RenderPass, RenderPlugin,
    ShaderPath, ShaderSource, StoreOp,
};
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
        .add_draw::<DrawMesh2d>()
        .embed_assets("embedded", embedded)
        .register::<Camera2d>()
        .register::<Mesh2dRenderer>()
        .add_asset(MATERIAL_ID, UnlitColor::new(Color::red()), vec![])
        .add_asset(MESH_ID, quad, vec![])
        .add_systems(Init, |actions: WorldActions| {
            actions.add(Spawn::new().with(Camera2d));
            actions.add(Spawn::new().with(Mesh2dRenderer::new(MESH_ID, MATERIAL_ID)));
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
pub struct Camera2d;

#[derive(ShaderType, Clone, Copy)]
pub struct Camera2dView {
    pub world: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

impl render::View for Camera2dView {
    type Query = (Entity, Read<Camera2d>);

    fn world(&self) -> glam::Mat4 {
        self.world
    }

    fn view(&self) -> glam::Mat4 {
        self.view
    }

    fn projection(&self) -> glam::Mat4 {
        self.projection
    }

    fn extract<'a>(
        query: <Self::Query as ecs::prelude::query::BaseQuery>::Item<'a>,
    ) -> render::ExtractedView<Self> {
        let (entity, _) = query;
        render::ExtractedView {
            entity,
            view: Camera2dView {
                world: glam::Mat4::IDENTITY,
                view: glam::Mat4::IDENTITY,
                projection: glam::Mat4::IDENTITY,
            },
            depth: 0,
            viewport: None,
            clear_color: None,
        }
    }
}

pub struct UnlitColorPass;
impl DrawPass for UnlitColorPass {
    type View = Camera2dView;

    const NAME: render::renderer::Name = "UnlitColor";

    fn setup(builder: &mut render::renderer::PassBuilder) -> RenderPass {
        let surface = builder.write(builder.surface_id());
        let depth = builder.write(builder.resource_id(MainDrawPass::DEPTH_TEXTURE));

        RenderPass::new()
            .with_color(surface, None, StoreOp::Store, Some(Color::green()))
            .with_depth(
                depth,
                Operations {
                    load: render::LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                },
                None,
            )
    }
}

#[derive(Component, Clone, Copy)]
pub struct Mesh2dRenderer {
    pub mesh: AssetRef<render::Mesh>,
    pub material: AssetRef<UnlitColor>,
}

impl Mesh2dRenderer {
    pub fn new(
        mesh: impl Into<AssetRef<render::Mesh>>,
        material: impl Into<AssetRef<UnlitColor>>,
    ) -> Self {
        Self {
            mesh: mesh.into(),
            material: material.into(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DrawMesh2d {
    enitity: Entity,
    material: AssetRef<UnlitColor>,
    mesh: AssetRef<render::Mesh>,
    data: Mesh2dData,
}

impl Draw for DrawMesh2d {
    type View = Camera2dView;

    type Mesh = Mesh2dData;

    type Material = UnlitColor;

    type Pass = UnlitColorPass;

    type Query = (Entity, Read<Mesh2dRenderer>);

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
        let (entity, renderer) = query;
        Self {
            enitity: entity,
            material: renderer.material,
            mesh: renderer.mesh,
            data: Mesh2dData {
                model: glam::Mat4::IDENTITY,
            },
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
