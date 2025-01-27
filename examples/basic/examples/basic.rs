use uuid::Uuid;

const VERTEX_SHADER_ID: Uuid = Uuid::from_u128(0);
const FRAGMENT_SHADER_ID: Uuid = Uuid::from_u128(1);
const MATERIAL_ID: Uuid = Uuid::from_u128(0);
const MESH_ID: Uuid = Uuid::from_u128(1);

fn main() {
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
}

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
