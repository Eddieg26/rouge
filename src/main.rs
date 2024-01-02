use core::panic;
use std::collections::HashMap;

use game::Game;
use graphics::resources::shader::{
    layout::{BufferLayout, ShaderBinding, ShaderBindings, ShaderVariable},
    ShaderMeta,
};

use crate::graphics::resources::shader::{
    layout::{ShaderAttribute, ShaderDef},
    Shader,
};

pub mod asset;
pub mod ecs;
pub mod game;
pub mod graphics;
pub mod primitives;
pub mod tree;

fn main() {
    // let mut global_bindings = ShaderBindings::new();

    // let global_layout = BufferLayout::new(&[ShaderVariable::Mat4, ShaderVariable::Mat4]);

    // global_bindings.add_binding(ShaderBinding::UniformBuffer { layout: global_layout, count: None });

    // let shader_meta = ShaderMeta::new("fs_main", vec![ShaderBindings::new()], inputs, outputs)

    let shader_src = std::fs::read_to_string("instanced_vertex_shader.wgsl")
        .expect("Failed to read vertex shader");

    let module = naga::front::wgsl::parse_str(&shader_src).expect("Failed to parse shader");

    let mut bindings: HashMap<usize, ShaderBindings> = HashMap::new();
    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut entry_point = String::new();

    if let Some(entry) = module.entry_points.first() {
        entry_point = entry.name.clone();

        let function = &entry.function;

        for arg in function.arguments.iter() {
            let attribute = arg.binding.as_ref().map(|b| ShaderAttribute::from(b));

            let ty = &module.types[arg.ty];
            let variable = ShaderVariable::from_naga(&ty.inner, &module.types);

            inputs.push(ShaderDef::new(attribute, variable));
        }

        if let Some(result) = &entry.function.result {
            let ty = &module.types[result.ty];
            let variable = ShaderDef::from_naga(&result.binding, &ty.inner, &module.types);
            outputs.push(variable);
        }
    }

    for (_, global) in module.global_variables.iter() {
        let binding = if let Some(binding) = &global.binding {
            binding
        } else {
            continue;
        };

        let ty = &module.types[global.ty];
        let shader_binding = ShaderBinding::from_naga(&ty.inner, &global.space, &module.types);

        bindings
            .entry(binding.group as usize)
            .or_insert(ShaderBindings::with_group(binding.group))
            .insert_binding(binding.binding as usize, shader_binding);
    }

    let mut bindings = bindings
        .into_iter()
        .map(|(_, bindings)| bindings)
        .collect::<Vec<_>>();

    bindings.sort_by(|a, b| a.group().cmp(&b.group()));

    let shader_meta = ShaderMeta::new(&entry_point, bindings, inputs, outputs);

    println!("Shader Meta: {}\n", shader_meta);

    // for ty in module.types.iter() {
    //     println!("Type: {:?}", ty);
    // }

    // Game::new().run();
}
