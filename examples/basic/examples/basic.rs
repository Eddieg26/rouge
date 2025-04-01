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
    wgpu::RenderPassDescriptor,
    Color, Draw, DrawFunctions, ExtractedView, IntoDrawCall, Material, MaterialPhase, MaterialRef,
    Mesh, MeshAttribute, MeshAttributeType, MeshAttributeValues, MeshTopology, PassBuilder,
    RenderAppExt, RenderAssets, RenderContext, RenderMesh, RenderPlugin, RenderState, RenderTarget,
    RenderView, Renderer, ShaderPath, ShaderSource, SubGraph, ViewBuffer, ViewData, ViewDrawCalls,
    ViewPass, ViewPassNode,
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
        .register::<Camera2d>()
        .register::<Mesh2d>()
        .add_plugin(RenderPlugin)
        .register_asset::<UnlitColor>()
        .add_sub_graph(Render2d)
        .add_sub_graph_pass(Render2d, ViewPassNode::<Main2dPass>::new())
        .embed_assets("embedded", embedded)
        .add_draw::<DrawMesh2d<UnlitColor>>()
        .add_material_phase::<Camera2dView, Opaque2d>()
        .add_material_phase::<Camera2dView, Transparent2d>()
        .add_asset(MESH_ID, quad, vec![])
        .add_asset(MATERIAL_ID, UnlitColor::new(Color::red()), vec![])
        .add_systems(Init, |actions: WorldActions| {
            actions.add(
                Spawn::new()
                    .with(Camera2d { depth: 0 })
                    .with(Transform::default()),
            );
            actions.add(
                Spawn::new()
                    .with(Mesh2d::new(MESH_ID))
                    .with(Transform::default())
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

#[derive(Clone, Copy)]
pub struct Opaque2d;

impl MaterialPhase for Opaque2d {
    type Item = Self;

    fn mode() -> render::BlendMode {
        render::BlendMode::Opaque
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZOrder(pub i32);

#[derive(Clone, Copy)]
pub struct Transparent2d {
    pub z_order: ZOrder,
}

impl MaterialPhase for Transparent2d {
    type Item = Self;

    fn mode() -> render::BlendMode {
        render::BlendMode::Transparent
    }
}

impl Material for UnlitColor {
    type Phase = Opaque2d;

    fn shader() -> impl Into<ShaderPath> {
        FRAGMENT_SHADER_ID
    }
}

#[derive(Clone, Copy, Component)]
pub struct Camera2d {
    depth: i32,
}

#[derive(Clone, Copy, ShaderType)]
pub struct Camera2dView {
    pub world: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

impl ViewData for Camera2dView {
    type Query = (Entity, Read<Transform>, Read<Camera2d>);

    fn world(&self) -> &glam::Mat4 {
        &self.world
    }

    fn extract<'a>(
        query: <Self::Query as ecs::prelude::query::BaseQuery>::Item<'a>,
    ) -> ExtractedView<Self> {
        let (entity, transform, camera) = query;
        ExtractedView {
            entity,
            view: Camera2dView {
                world: transform.world(),
                view: transform.world().inverse(),
                projection: glam::Mat4::IDENTITY,
            },
            depth: camera.depth,
        }
    }
}

pub struct Mesh2dRenderer;

#[derive(Clone, Copy, ShaderType)]
pub struct MeshData2d {
    pub world: glam::Mat4,
}

impl Renderer for Mesh2dRenderer {
    type View = Camera2dView;

    type Mesh = MeshData2d;

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

    fn shader() -> impl Into<ShaderPath> {
        VERTEX_SHADER_ID
    }
}

#[derive(Clone, Copy, Component)]
pub struct Mesh2d(pub AssetRef<Mesh>);
impl Mesh2d {
    pub fn new(mesh: impl Into<AssetRef<Mesh>>) -> Self {
        Self(mesh.into())
    }
}
impl std::ops::Deref for Mesh2d {
    type Target = AssetRef<Mesh>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Mesh2d {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct DrawMesh2d<M: Material> {
    pub entity: Entity,
    pub material: AssetRef<M>,
    pub mesh: AssetRef<Mesh>,
    pub data: MeshData2d,
    pub z_order: ZOrder,
}

impl<M: Material> IntoDrawCall<Opaque2d> for DrawMesh2d<M> {
    fn into_draw_call(&self, _: &spatial::RangeFinder) -> <Opaque2d as MaterialPhase>::Item {
        Opaque2d
    }
}

impl<M: Material> IntoDrawCall<Transparent2d> for DrawMesh2d<M> {
    fn into_draw_call(&self, _: &spatial::RangeFinder) -> <Transparent2d as MaterialPhase>::Item {
        Transparent2d {
            z_order: self.z_order,
        }
    }
}

impl<M: Material> Draw for DrawMesh2d<M> {
    type View = Camera2dView;

    type Material = M;

    type Renderer = Mesh2dRenderer;

    type Query = (Entity, Read<Transform>, Read<Mesh2d>, Read<MaterialRef<M>>);

    fn entity(&self) -> Entity {
        self.entity
    }

    fn data(&self) -> render::DrawMesh<Self> {
        self.data
    }

    fn material(&self) -> AssetRef<Self::Material> {
        self.material
    }

    fn mesh(&self) -> AssetRef<Mesh> {
        self.mesh
    }

    fn extract<'a>(query: <Self::Query as ecs::prelude::query::BaseQuery>::Item<'a>) -> Self {
        let (entity, transform, mesh, material) = query;
        DrawMesh2d {
            entity,
            material: **material,
            mesh: **mesh,
            data: MeshData2d {
                world: transform.world(),
            },
            z_order: ZOrder(0),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Render2d;

impl SubGraph for Render2d {
    const NAME: render::Name = "Render2d";
}

pub struct Main2dPass;

impl ViewPass for Main2dPass {
    type View = Camera2dView;

    const NAME: render::renderer::Name = "Main2dPass";

    fn setup(
        builder: &mut PassBuilder,
    ) -> impl Fn(&mut RenderContext, &RenderView<Self::View>, &ViewBuffer<Self::View>) + 'static
    {
        builder.write::<RenderTarget>();

        |ctx, view, view_buffer| {
            let mut encoder = ctx.encoder();
            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some(Self::NAME),
                    color_attachments: &[],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                let mut state = RenderState::new(&mut pass);

                let functions = ctx.world().resource::<DrawFunctions<Self::View>>();
                let meshes = ctx.world().resource::<RenderAssets<RenderMesh>>();

                let opaque = ctx
                    .world()
                    .resource::<ViewDrawCalls<Self::View, Opaque2d>>();

                let transparent = ctx
                    .world()
                    .resource::<ViewDrawCalls<Self::View, Transparent2d>>();

                opaque.draw(&mut state, ctx, functions, meshes, view_buffer, view);
                transparent.draw(&mut state, ctx, functions, meshes, view_buffer, view);
            }
            
            ctx.submit(encoder.finish());
        }
    }
}
