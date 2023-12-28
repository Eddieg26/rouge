use std::io::Write;

use game::Game;
use graphics::resources::shader::graph::{
    attribute::Attribute, nodes::SampleTexture2D, ShaderGraph, Slot,
};

pub mod ecs;
pub mod game;
pub mod graphics;
pub mod primitives;
pub mod tree;

fn main() {
    let mut graph = ShaderGraph::default();
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
