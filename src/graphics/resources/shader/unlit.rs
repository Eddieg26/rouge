use crate::graphics::resources::{
    material::Material,
    shader::templates::{BuiltinName, FieldKind, Uniform},
};

use super::ShaderTemplate;

pub struct UnlitShaderTemplate;

impl ShaderTemplate for UnlitShaderTemplate {
    fn create_shader(device: &wgpu::Device, material: &Material) -> super::Shader {
        let mut global_uniform = Uniform::new("Global");
        global_uniform.add_field("view", FieldKind::Mat4);
        global_uniform.add_field("projection", FieldKind::Mat4);

        let mut object_uniform = Uniform::new("Object");
        object_uniform.add_field("model", FieldKind::Mat4);

        let material_uniform = Uniform::from_material(material);

        let mut vertex_input = Uniform::new("VertexInput");
        vertex_input.add_builtin_field("position", FieldKind::Vec3, BuiltinName::Position);
        vertex_input.add_field("normal", FieldKind::Vec3);
        vertex_input.add_field("uv", FieldKind::Vec2);

        let mut vertex_output = Uniform::new("VertexOutput");
        vertex_output.add_builtin_field("position", FieldKind::Vec4, BuiltinName::Position);

        let mut template = String::new();
        template.push_str(&format!(
            r#"
                {global}
                {object}

                {vs_in}
                {vs_out}
                
                {global_binding}
                {object_binding}

                @vertex
                fn vs_main(in: {vs_in_name}) -> {vs_out_name} {{
                    let out = {vs_out_name};

                    out.position = {global_binding}.view * mul({global_binding}.projection * {object_binding}.model * in.position;
                    out.normal = in.normal;
                    out.uv = in.uv;

                    return out;
                }}

                {standard}
                {mat}

                {material_bindings}
                {texture_bindings}
                {sampler_bindings}

                @fragment
                fn fs_main(in: {vs_out_name}) -> @location(0) vec4<f32> {{
                    let material = {standard_name};

                    {texture_values}
                    {material_values}

                    return vec4<f32>(material.color.xyz, {opacity});
                }}

            "#,
            global = global_uniform.create_def(),
            object = object_uniform.create_def(),
            vs_in = vertex_input.create_def(),
        ));

        todo!()
    }
}
