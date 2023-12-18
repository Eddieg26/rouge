use std::io::Write;

use game::Game;
use graphics::{
    plugin::GraphicsPlugin,
    renderer::simple::SimpleRendererPlugin,
    resources::{
        material::{BlendMode, Material, ShaderInput, ShaderModel},
        shader::{
            graph::{Attribute, ShaderGraph, Slot},
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
    // let mut material = Material::new(ShaderModel::Unlit);
    // material.set_color(ShaderInput::Texture(TextureId::zero()));
    // material.set_blend_mode(BlendMode::Transparent(ShaderInput::Texture(
    //     TextureId::zero(),
    // )));

    // let shader_src = UnlitShaderTemplate::create_source(&material);

    // let mut file = std::fs::File::create("unlit-texture.wgsl").unwrap();
    // file.write_all(shader_src.as_bytes()).unwrap();

    let mut graph = ShaderGraph::new();
    graph.add_input("color", Attribute::Color);
    graph.add_output("out_color", 0);
    graph.add_edge(Slot::new("color", 0), Slot::new("out_color", 0));

    let shader_src = graph.generate();

    let mut file = std::fs::File::create("unlit-texture-graph.wgsl").unwrap();
    file.write_all(shader_src.as_bytes()).unwrap();
    // Game::new().add_plugin(SimpleRendererPlugin).run();
}
