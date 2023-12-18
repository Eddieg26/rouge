use std::io::Write;

use game::Game;
use graphics::{
    plugin::GraphicsPlugin,
    renderer::simple::SimpleRendererPlugin,
    resources::{
        material::{BlendMode, Material, ShaderInput, ShaderModel},
        shader::{
            graph::{nodes::SampleTexture2D, Attribute, ShaderGraph, Slot},
            unlit::UnlitShaderTemplate,
        },
        TextureId,
    },
};

pub mod ecs;
pub mod game;
pub mod graphics;
pub mod primitives;
pub mod tree;

fn main() {
    let mut graph = ShaderGraph::new();
    // graph.add_input("color", Attribute::Color);
    graph.add_input("texture", Attribute::Texture2D);
    graph.add_input("uv", Attribute::Vec2);
    graph.add_output("out_color", 0);
    graph.add_node(SampleTexture2D::new("sample_texture"));
    graph.add_edge(
        Slot::new("texture", 0),
        Slot::new("sample_texture", SampleTexture2D::TEXTURE_SLOT),
    );
    graph.add_edge(
        Slot::new("uv", 0),
        Slot::new("sample_texture", SampleTexture2D::UV_SLOT),
    );
    graph.add_edge(Slot::new("sample_texture", 0), Slot::new("out_color", 0));

    let shader_src = graph.generate();

    let mut file = std::fs::File::create("unlit-texture-graph-2.wgsl").unwrap();
    file.write_all(shader_src.as_bytes()).unwrap();
    // Game::new().add_plugin(SimpleRendererPlugin).run();
}
