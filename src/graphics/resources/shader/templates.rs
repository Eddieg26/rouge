use crate::graphics::resources::material::{
    BlendMode, InputNames, Material, ShaderInput, ShaderModel,
};
use itertools::Itertools;

pub enum FieldKind {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
}

impl FieldKind {
    pub fn type_name(&self) -> &str {
        match self {
            FieldKind::Scalar => "f32",
            FieldKind::Vec2 => "vec2<f32>",
            FieldKind::Vec3 => "vec3<f32>",
            FieldKind::Vec4 => "vec4<f32>",
            FieldKind::Mat2 => "mat2x2f",
            FieldKind::Mat3 => "mat3x3f",
            FieldKind::Mat4 => "mat4x4f",
        }
    }

    pub fn size(&self) -> u32 {
        match self {
            FieldKind::Scalar => 1,
            FieldKind::Vec2 => 2,
            FieldKind::Vec3 => 3,
            FieldKind::Vec4 => 4,
            FieldKind::Mat2 => 4,
            FieldKind::Mat3 => 9,
            FieldKind::Mat4 => 16,
        }
    }

    pub fn convert(
        src: &FieldKind,
        dst: &FieldKind,
        src_name: &str,
        dst_name: &str,
    ) -> Option<String> {
        match (dst, src) {
            (FieldKind::Scalar, FieldKind::Vec2) => Some(format!(
                "{dst} = ({src}).x;\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Scalar, FieldKind::Vec3) => Some(format!(
                "{dst} = ({src}).x;\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Scalar, FieldKind::Vec4) => Some(format!(
                "{dst} = ({src}).x;\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec2, FieldKind::Scalar) => Some(format!(
                "{dst} = vec2<f32>({src});\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec3, FieldKind::Scalar) => Some(format!(
                "{dst} = vec3<f32>({src});\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec4, FieldKind::Scalar) => Some(format!(
                "{dst} = vec4<f32>({src});\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec2, FieldKind::Vec3) => Some(format!(
                "{dst} = vec2<f32>({src}.xy);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec2, FieldKind::Vec4) => Some(format!(
                "{dst} = vec2<f32>({src}.xy);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec3, FieldKind::Vec2) => Some(format!(
                "{dst} = vec3<f32>({src}, 0.0);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec3, FieldKind::Vec4) => Some(format!(
                "{dst} = vec3<f32>({src}.xyz);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec4, FieldKind::Vec2) => Some(format!(
                "{dst} = vec4<f32>({src}, 0.0, 0.0);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Vec4, FieldKind::Vec3) => Some(format!(
                "{dst} = vec4<f32>({src}.xyz, 0.0);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat2, FieldKind::Mat3) => Some(format!(
                "{dst} = mat2x2<f32>({src}.xy, {src}.zw);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat2, FieldKind::Mat4) => Some(format!(
                "{dst} = mat2x2<f32>({src}.xy, {src}.zw);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat3, FieldKind::Mat2) => Some(format!(
                "{dst} = mat3x3<f32>({src}.xy, 0.0, {src}.zw, 0.0, 0.0, 1.0);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat3, FieldKind::Mat4) => Some(format!(
                "{dst} = mat3x3<f32>({src}.xyz, {src}.w);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat4, FieldKind::Mat2) => Some(format!(
                "{dst} = mat4x4<f32>({src}.xy, 0.0, 0.0, {src}.zw, 0.0, 0.0, 0.0, 1.0);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat4, FieldKind::Mat3) => Some(format!(
                "{dst} = mat4x4<f32>({src}.xyz, 0.0, {src}.w);\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Mat2, FieldKind::Mat2) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Mat3, FieldKind::Mat3) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Mat4, FieldKind::Mat4) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Vec4, FieldKind::Vec4) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Vec3, FieldKind::Vec3) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Vec2, FieldKind::Vec2) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }
            (FieldKind::Scalar, FieldKind::Scalar) => {
                Some(format!("{dst} = {src};\n", dst = dst_name, src = src_name))
            }

            _ => None,
        }
    }
}

pub enum Attribute {
    Location(u32),
    VertexIndex,
    InstanceIndex,
    Position,
    FrontFacing,
    FragDepth,
    SampleIndex,
    SampleMask,
    LocalInvocationId,
    LocalInvocationIndex,
    GlobalInvocationId,
    WorkgroupId,
    NumWorkgroups,
}

impl Attribute {
    pub fn name(&self) -> String {
        match self {
            Attribute::Location(idx) => format!("@location({})", idx),
            Attribute::VertexIndex => String::from("@builtin(vertex_index)"),
            Attribute::InstanceIndex => String::from("@builtin(instance_index)"),
            Attribute::Position => String::from("@builtin(position)"),
            Attribute::FrontFacing => String::from("@builtin(front_facing)"),
            Attribute::FragDepth => String::from("@builtin(frag_depth)"),
            Attribute::SampleIndex => String::from("@builtin(sample_index)"),
            Attribute::SampleMask => String::from("@builtin(sample_mask)"),
            Attribute::LocalInvocationId => String::from("@builtin(local_invocation_id)"),
            Attribute::LocalInvocationIndex => String::from("@builtin(local_invocation_index)"),
            Attribute::GlobalInvocationId => String::from("@builtin(global_invocation_id)"),
            Attribute::WorkgroupId => String::from("@builtin(work_group_id)"),
            Attribute::NumWorkgroups => String::from("@builtin(num_work_groups)"),
        }
    }
}

pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub attribute: Option<Attribute>,
}

impl Field {
    pub fn new(name: impl Into<String>, kind: FieldKind, attribute: Option<Attribute>) -> Self {
        Self {
            name: name.into(),
            kind,
            attribute,
        }
    }

    fn from_input(name: &str, input: &ShaderInput) -> Option<Field> {
        match input {
            ShaderInput::Scalar(_) => Some(Field::new(name.to_string(), FieldKind::Scalar, None)),
            ShaderInput::Color(_) => Some(Field::new(name.to_string(), FieldKind::Vec4, None)),
            ShaderInput::Texture(_) => None,
        }
    }
}

pub struct Uniform {
    type_name: String,
    fields: Vec<Field>,
}

impl Uniform {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            fields: Vec::new(),
        }
    }

    pub fn from_material(material: &Material) -> Uniform {
        let mut uniform =
            Uniform::new("Material").add_optional_input(InputNames::COLOR, material.color());

        match material.shader_model() {
            ShaderModel::Lit {
                normal,
                specular,
                metallic,
                roughness,
                emission,
            } => {
                uniform = uniform
                    .add_optional_input(InputNames::NORMAL, normal)
                    .add_optional_input(InputNames::EMISSION, emission)
                    .add_optional_input(InputNames::METALLIC, metallic)
                    .add_optional_input(InputNames::ROUGHNESS, roughness)
                    .add_optional_input(InputNames::SPECULAR, specular);
            }
            ShaderModel::Unlit => {}
        }

        match material.blend_mode() {
            BlendMode::Opaque => {}
            BlendMode::Transparent(input) => {
                uniform = uniform.add_optional_input(InputNames::OPACITY, &input);
            }
        }

        uniform
    }

    pub fn size(&self) -> u32 {
        self.fields.iter().map(|f| f.kind.size()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    fn add_optional_input(self, name: &str, input: &ShaderInput) -> Self {
        if let Some(field) = Field::from_input(name, input) {
            return self.add_field(&field.name, field.kind, None);
        }

        self
    }

    pub fn add_field(mut self, name: &str, kind: FieldKind, attribute: Option<Attribute>) -> Self {
        self.fields.push(Field::new(name, kind, attribute));
        self
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn create_def(&self) -> String {
        let mut def = String::new();
        if self.fields.is_empty() {
            return def;
        }

        def.push_str(&format!("struct {} {{\n", self.type_name));
        for field in &self.fields {
            let attribute = field
                .attribute
                .as_ref()
                .map(|b| b.name())
                .unwrap_or(String::new());
            def.push_str(&format!(
                "{} {}: {};\n",
                attribute,
                field.name,
                field.kind.type_name()
            ));
        }
        def.push_str("};\n");

        def
    }

    pub fn create_binding(&self, group: u32, binding: u32, name: &str) -> String {
        if self.fields.is_empty() {
            return String::new();
        }

        format!(
            r#"@group({}) @binding({})
                var {}: {};
            "#,
            group, binding, name, self.type_name
        )
    }

    pub fn create_field_values(&self, src: &str, dst: &str, fields: &[Field]) -> String {
        if self.fields.is_empty() {
            return String::new();
        }

        let mut values = String::new();

        for field in fields {
            if let Some(src_field) = self.field(&field.name) {
                let src_name = format!("{}.{}", src, field.name);
                let dst_name = format!("{}.{}", dst, field.name);
                let value = FieldKind::convert(&src_field.kind, &field.kind, &src_name, &dst_name)
                    .expect(&format!(
                        "Could not convert {} field: {} to {}",
                        field.name,
                        src_field.kind.type_name(),
                        field.kind.type_name()
                    ));
                values.push_str(&value);
            }
        }

        values
    }
}

#[derive(Clone)]
pub struct TextureBinding(String);

impl TextureBinding {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn create_binding(&self, group: u32, binding: u32) -> String {
        format!(
            r#"@group({}) @binding({})
                var {}_texture: texture_2d<f32>;
            "#,
            group, binding, self.0
        )
    }

    pub fn get_field_value(&self, uniform: &str, field: &Field) -> String {
        let dst_name = format!("{}.{}", uniform, field.name);
        let src_name = format!(
            "textureSample({name}_texture, {name}_sampler, in.uv.xy)",
            name = self.name()
        );
        let src_field = Field::new(&src_name, FieldKind::Vec4, None);
        FieldKind::convert(&src_field.kind, &field.kind, &src_field.name, &dst_name)
            .unwrap()
            .to_string()
    }

    pub fn from_material(material: &Material) -> Vec<TextureBinding> {
        let mut bindings = vec![];
        bindings.push(Self::from_input(InputNames::COLOR, material.color()));
        match material.shader_model() {
            ShaderModel::Lit {
                normal,
                specular,
                metallic,
                roughness,
                emission,
            } => {
                bindings.push(Self::from_input(InputNames::NORMAL, normal));
                bindings.push(Self::from_input(InputNames::SPECULAR, specular));
                bindings.push(Self::from_input(InputNames::METALLIC, metallic));
                bindings.push(Self::from_input(InputNames::ROUGHNESS, roughness));
                bindings.push(Self::from_input(InputNames::EMISSION, emission));
            }
            ShaderModel::Unlit => {}
        }

        match material.blend_mode() {
            BlendMode::Opaque => {}
            BlendMode::Transparent(opacity) => {
                bindings.push(Self::from_input(InputNames::OPACITY, opacity));
            }
        }

        bindings.iter().filter_map(|i| i.clone()).collect_vec()
    }

    pub fn from_input(name: &str, input: &ShaderInput) -> Option<TextureBinding> {
        match input {
            ShaderInput::Texture(_) => Some(TextureBinding(name.to_string())),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct SamplerBinding(String);

impl SamplerBinding {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn create_binding(&self, group: u32, binding: u32) -> String {
        format!(
            r#"@group({}) @binding({})
                var {}_sampler: sampler;
            "#,
            group, binding, self.0
        )
    }

    pub fn from_textures(textures: &[TextureBinding]) -> Vec<SamplerBinding> {
        textures
            .iter()
            .map(|texture| SamplerBinding(texture.0.clone()))
            .collect_vec()
    }
}

pub struct ShaderBindGroup {
    uniforms: Vec<Uniform>,
    textures: Vec<TextureBinding>,
    samplers: Vec<SamplerBinding>,
}

impl ShaderBindGroup {
    pub fn new() -> Self {
        Self {
            uniforms: Vec::new(),
            textures: Vec::new(),
            samplers: Vec::new(),
        }
    }

    pub fn from_material(material: &Material) -> ShaderBindGroup {
        let textures = TextureBinding::from_material(material);
        let samplers = SamplerBinding::from_textures(&textures);

        Self {
            uniforms: vec![],
            textures,
            samplers,
        }
    }

    pub fn add_uniform(&mut self, uniform: Uniform) -> &mut Self {
        self.uniforms.push(uniform);
        self
    }

    pub fn add_texture(&mut self, texture: TextureBinding) -> &mut Self {
        self.textures.push(texture);
        self
    }

    pub fn add_sampler(&mut self, sampler: SamplerBinding) -> &mut Self {
        self.samplers.push(sampler);
        self
    }

    pub fn uniforms(&self) -> &[Uniform] {
        &self.uniforms
    }

    pub fn textures(&self) -> &[TextureBinding] {
        &self.textures
    }

    pub fn samplers(&self) -> &[SamplerBinding] {
        &self.samplers
    }

    fn texture(&self, name: &str) -> Option<&TextureBinding> {
        self.textures.iter().find(|t| t.name() == name)
    }

    pub fn create_bindings(&self, group: u32, start_binding: u32) -> String {
        let mut bindings = String::new();
        let mut bind_idx = start_binding;

        for uniform in &self.uniforms {
            bindings.push_str(&format!(
                "{}",
                uniform.create_binding(group, bind_idx, &uniform.type_name)
            ));
            bind_idx += 1;
        }

        for texture in &self.textures {
            bindings.push_str(&format!("{}", texture.create_binding(group, bind_idx)));
            bind_idx += 1;
        }

        for sampler in &self.samplers {
            bindings.push_str(&format!("{}", sampler.create_binding(group, bind_idx)));
            bind_idx += 1;
        }

        bindings
    }

    pub fn create_field_values(&self, name: &str, fields: &[Field]) -> String {
        let mut values = String::new();

        for field in fields {
            if let Some(binding) = self.texture(&field.name) {
                values.push_str(&binding.get_field_value(name, field))
            }
        }

        values
    }
}

pub mod common {
    use super::{Attribute, FieldKind, Uniform};

    pub fn vertex_input() -> Uniform {
        Uniform::new("VertexInput")
            .add_field("position", FieldKind::Vec3, Some(Attribute::Location(0)))
            .add_field("normal", FieldKind::Vec3, Some(Attribute::Location(1)))
            .add_field("uv", FieldKind::Vec2, Some(Attribute::Location(2)))
    }

    pub fn vertex_output() -> Uniform {
        Uniform::new("VertexOutput")
            .add_field("position", FieldKind::Vec4, Some(Attribute::Position))
            .add_field("normal", FieldKind::Vec3, Some(Attribute::Location(0)))
            .add_field("uv", FieldKind::Vec2, Some(Attribute::Location(1)))
    }
}
