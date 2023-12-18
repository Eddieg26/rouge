use crate::graphics::resources::{
    material::{BlendMode, Material},
    shader::{
        templates::{self, FieldKind, ShaderBindGroup, Uniform},
        Shader,
    },
};

use super::ShaderTemplate;

impl UnlitShaderTemplate {
    pub fn create_source(material: &Material) -> String {
        let vertex_output = templates::common::vertex_output();

        let material_uniform = Uniform::from_material(material);
        let standard_uniform = Uniform::new("Standard")
            .add_field("color", FieldKind::Vec4, None)
            .add_field("opacity", FieldKind::Scalar, None);

        let shader_bindings = ShaderBindGroup::from_material(material);

        let opacity = match material.blend_mode() {
            BlendMode::Opaque => "1.0",
            BlendMode::Transparent(_) => "standard.opacity",
        };

        let start_binding = if material_uniform.is_empty() { 0 } else { 1 };

        let mut template = String::new();
        template.push_str(&format!(
            r#"
                {vs_out}

                {standard}
                {mat}

                {material_bindings}
                {shader_bindings}

                @fragment
                fn fs_main(in: {vs_out_name}) -> @location(0) vec4<f32> {{
                    let standard: {standard_name};

                    {texture_values}
                    {material_values}

                    return vec4<f32>(standard.color.xyz, {opacity});
                }}

            "#,
            vs_out = vertex_output.create_def(),
            vs_out_name = vertex_output.type_name(),
            standard = standard_uniform.create_def(),
            mat = material_uniform.create_def(),
            material_bindings =
                material_uniform.create_binding(Self::MATERIAL_BIND_GROUP, 0, "material"),
            shader_bindings =
                shader_bindings.create_bindings(Self::MATERIAL_BIND_GROUP, start_binding),
            standard_name = standard_uniform.type_name(),
            texture_values =
                shader_bindings.create_field_values("standard", standard_uniform.fields()),
            material_values = material_uniform.create_field_values(
                "material",
                "standard",
                standard_uniform.fields()
            ),
            opacity = opacity
        ));

        template
    }
}

pub struct UnlitShaderTemplate;

impl ShaderTemplate for UnlitShaderTemplate {
    const MATERIAL_BIND_GROUP: u32 = 1;
    fn create_shader(device: &wgpu::Device, material: &Material) -> super::Shader {
        let vertex_output = templates::common::vertex_output();

        let material_uniform = Uniform::from_material(material);
        let standard_uniform = Uniform::new("Standard")
            .add_field("color", FieldKind::Vec4, None)
            .add_field("opacity", FieldKind::Scalar, None);

        let shader_bindings = ShaderBindGroup::from_material(material);

        let opacity = match material.blend_mode() {
            BlendMode::Opaque => "1.0",
            BlendMode::Transparent(_) => "standard.opacity",
        };

        let mut template = String::new();
        template.push_str(&format!(
            r#"
                {vs_out}

                {standard}
                {mat}

                {material_bindings}
                {shader_bindings}

                @fragment
                fn fs_main(in: {vs_out_name}) -> @location(0) vec4<f32> {{
                    let standard: {standard_name};

                    {texture_values}
                    {material_values}

                    return vec4<f32>(standard.color.xyz, {opacity});
                }}

            "#,
            vs_out = vertex_output.create_def(),
            vs_out_name = vertex_output.type_name(),
            standard = standard_uniform.create_def(),
            mat = material_uniform.create_def(),
            material_bindings =
                material_uniform.create_binding(Self::MATERIAL_BIND_GROUP, 0, "material"),
            shader_bindings = shader_bindings.create_bindings(Self::MATERIAL_BIND_GROUP, 1),
            standard_name = standard_uniform.type_name(),
            texture_values =
                shader_bindings.create_field_values("standard", standard_uniform.fields()),
            material_values = material_uniform.create_field_values(
                "material",
                "standard",
                standard_uniform.fields()
            ),
            opacity = opacity
        ));

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(template.into()),
        });

        let mut entries = vec![];
        if !material_uniform.is_empty() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: entries.len() as u32,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
            });
        }

        for _ in shader_bindings.textures() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: (entries.len() + 1) as u32,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
            });
        }

        for _ in shader_bindings.textures() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: (entries.len() + 1) as u32,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
            });
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });

        let buffer_size = material_uniform.size();

        Shader::new(
            "fs_main",
            shader_module,
            layout,
            Self::MATERIAL_BIND_GROUP,
            buffer_size,
        )
    }
}
