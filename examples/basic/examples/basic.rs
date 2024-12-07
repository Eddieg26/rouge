use asset::{
    database::{events::AssetEvent, AssetDatabase},
    embed_asset,
    importer::{DefaultProcessor, ImportContext, ImportError, Importer},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache, LoadPath},
        embedded::EmbeddedFs,
        local::LocalFs,
        source::{AssetPath, AssetSource},
        vfs::VirtualFs,
        AssetIoError, FileSystem,
    },
    plugin::{AssetExt, AssetPlugin},
    Asset, AssetId, AssetRef, Assets, AsyncReadExt, AsyncWriteExt,
};
use graphics::{
    core::{Color, ExtractError},
    encase::ShaderType,
    plugin::{RenderApp, RenderPlugin},
    renderer::{
        graph::{RenderGraphBuilder, RenderGraphNode},
        pass::{Attachment, LoadOp, Operations, RenderPass, StoreOp},
    },
    resource::{
        plugin::MaterialAppExt, BindGroupLayout, BindGroupLayoutBuilder, BlendMode, Material,
        MeshPipeline, MeshPipelineData, ShaderSource, Unlit, VertexAttribute,
    },
    wgpu::{PrimitiveState, ShaderStages},
    CreateBindGroup,
};
use std::{future::Future, path::PathBuf};
use uuid::Uuid;
use window::plugin::WindowPlugin;
// use asset::{
//     asset::{Asset, AssetId, AssetType},
//     io::{embed::EmbeddedFS, local::LocalFS, AssetSourceConfig},
// };
use ecs::{
    core::{
        component::Component,
        entity::Entity,
        resource::{Res, Resource},
    },
    event::{Event, Events},
    system::systems::Root,
    world::{
        self,
        action::WorldAction,
        cell::WorldCell,
        query::{Not, Query},
        World,
    },
};
use game::{Game, PostInit, Update};
use pollster::block_on;

const VERTEX_SHADER_ID: Uuid = Uuid::from_u128(0);
const FRAGMENT_SHADER_ID: Uuid = Uuid::from_u128(1);
const MATERIAL_ID: Uuid = Uuid::from_u128(0);

fn main() {
    let embedded = EmbeddedFs::new("assets");
    let vs_id = AssetRef::<ShaderSource>::from(VERTEX_SHADER_ID);
    embed_asset!(embedded, vs_id, "assets/vertex.wgsl", ());
    let fs_id = AssetRef::<ShaderSource>::from(FRAGMENT_SHADER_ID);
    embed_asset!(embedded, fs_id, "assets/fragment.wgsl", ());
    let mat_id = AssetId::from::<UnlitColor>(MATERIAL_ID);

    Game::new()
        .add_plugin(RenderPlugin)
        .add_material::<UnlitColor>()
        .add_asset(mat_id, UnlitColor::from(Color::green()), vec![])
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
        .embed_assets("basic", embedded)
        .add_systems(PostInit, |db: Res<AssetDatabase>| {
            db.load(["basic://assets/embedded.txt"]);
        })
        .run();
}

pub struct BasicMeshPipeline {
    layout: BindGroupLayout,
}

impl MeshPipelineData for BasicMeshPipeline {
    fn new(device: &graphics::RenderDevice) -> Self {
        let layout = BindGroupLayoutBuilder::new()
            .with_uniform_buffer(
                0,
                ShaderStages::VERTEX,
                true,
                Some(glam::Mat4::min_size()),
                None,
            )
            .build(device);

        Self { layout }
    }

    fn bind_group_layout(&self) -> &graphics::resource::BindGroupLayout {
        &self.layout
    }
}

impl MeshPipeline for BasicMeshPipeline {
    type Data = Self;

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
    type Meta = Unlit;

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
        if let Some(_) = self.pass.begin(ctx.target(), ctx, None, &mut encoder) {}

        ctx.submit(encoder);
    }
}
