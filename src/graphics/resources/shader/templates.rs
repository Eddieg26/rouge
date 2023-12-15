use crate::graphics::resources::material::{
    BlendMode, InputNames, Material, ShaderInput, ShaderModel,
};

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
            FieldKind::Vec2 => "vec2f",
            FieldKind::Vec3 => "vec3f",
            FieldKind::Vec4 => "vec4f",
            FieldKind::Mat2 => "mat2x2f",
            FieldKind::Mat3 => "mat3x3f",
            FieldKind::Mat4 => "mat4x4f",
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
                "{dst} = vec2<f32>({src}).x;\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Scalar, FieldKind::Vec3) => Some(format!(
                "{dst} = vec3<f32>({src}).x;\n",
                dst = dst_name,
                src = src_name
            )),
            (FieldKind::Scalar, FieldKind::Vec4) => Some(format!(
                "{dst} = vec4<f32>({src}).x;\n",
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
            _ => None,
        }
    }
}

pub enum BuiltinName {
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

impl BuiltinName {
    pub fn name(&self) -> &str {
        match self {
            BuiltinName::VertexIndex => "@builtin(vertex_index)",
            BuiltinName::InstanceIndex => "@builtin(instance_index)",
            BuiltinName::Position => "@builtin(position)",
            BuiltinName::FrontFacing => "@builtin(front_facing)",
            BuiltinName::FragDepth => "@builtin(frag_depth)",
            BuiltinName::SampleIndex => "@builtin(sample_index)",
            BuiltinName::SampleMask => "@builtin(sample_mask)",
            BuiltinName::LocalInvocationId => "@builtin(local_invocation_id)",
            BuiltinName::LocalInvocationIndex => "@builtin(local_invocation_index)",
            BuiltinName::GlobalInvocationId => "@builtin(global_invocation_id)",
            BuiltinName::WorkgroupId => "@builtin(work_group_id)",
            BuiltinName::NumWorkgroups => "@builtin(num_work_groups)",
        }
    }
}

pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub builtin: Option<BuiltinName>,
}

impl Field {
    pub fn new(name: impl Into<String>, kind: FieldKind, builtin: Option<BuiltinName>) -> Self {
        Self {
            name: name.into(),
            kind,
            builtin,
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
        let mut uniform = Uniform::new("Material");
        match material.shader_model() {
            ShaderModel::Lit(model) => {
                uniform.add_optional_input(InputNames::COLOR, &model.color);
                uniform.add_optional_input(InputNames::NORMAL, &model.normal);
                uniform.add_optional_input(InputNames::EMISSION, &model.emission);
                uniform.add_optional_input(InputNames::METALLIC, &model.metallic);
                uniform.add_optional_input(InputNames::ROUGHNESS, &model.roughness);
                uniform.add_optional_input(InputNames::SPECULAR, &model.specular);

                match model.blend_mode {
                    BlendMode::Opaque => {}
                    BlendMode::Transparent(input) => {
                        uniform.add_optional_input(InputNames::SPECULAR, &input)
                    }
                }
            }
            ShaderModel::Unlit(model) => {
                uniform.add_optional_input(InputNames::COLOR, &model.color);
                match model.blend_mode {
                    BlendMode::Opaque => {}
                    BlendMode::Transparent(input) => {
                        uniform.add_optional_input(InputNames::SPECULAR, &input)
                    }
                }
            }
        }

        uniform
    }

    fn add_optional_input(&mut self, name: &str, input: &ShaderInput) {
        if let Some(field) = Field::from_input(name, input) {
            self.add_field(&field.name, field.kind);
        }
    }

    pub fn add_field(&mut self, name: &str, kind: FieldKind) -> &mut Self {
        self.fields.push(Field::new(name, kind, None));
        self
    }

    pub fn add_builtin_field(
        &mut self,
        name: &str,
        kind: FieldKind,
        builtin: BuiltinName,
    ) -> &mut Self {
        self.fields.push(Field::new(name, kind, Some(builtin)));
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
        if !self.fields.is_empty() {
            return def;
        }

        def.push_str(&format!("struct {} {{\n", self.type_name));
        for field in &self.fields {
            let builtin = field.builtin.as_ref().map(|b| b.name()).unwrap_or("");
            def.push_str(&format!(
                "{} {}: {};\n",
                builtin,
                field.name,
                field.kind.type_name()
            ));
        }
        def.push_str("};\n");

        def
    }

    pub fn create_binding(&self, group: u32, binding: u32, name: &str) -> String {
        if !self.fields.is_empty() {
            return String::new();
        }

        format!(
            r#"@group({}) @binding({})
                {}: {};
            "#,
            group, binding, name, self.type_name
        )
    }
}

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
                var {}_texture: texture2d<f32>;
            "#,
            group, binding, self.0
        )
    }

    pub fn get_field_value(&self, uniform: &str, field: &Field) -> String {
        let dst_name = format!("{}_{}", uniform, field.name);
        let src_name = format!(
            "textureSample({name}_texture, {name}_sampler, input.uv.xy)",
            name = self.name()
        );
        let src_field = Field::new(&src_name, FieldKind::Vec4, None);
        FieldKind::convert(&src_field.kind, &field.kind, &src_field.name, &dst_name)
            .unwrap()
            .to_string()
    }
}

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

    pub fn create_def(&self) -> String {
        let mut def = String::new();
        for uniform in &self.uniforms {
            def.push_str(&uniform.create_def());
        }

        def
    }

    pub fn create_bindings(&self, group: u32) -> String {
        let mut bindings = String::new();
        for uniform in &self.uniforms {
            bindings.push_str(&format!(
                "{}",
                uniform.create_binding(group, bindings.len() as u32, &uniform.type_name)
            ));
        }

        for texture in &self.textures {
            bindings.push_str(&format!(
                "{}",
                texture.create_binding(group, bindings.len() as u32)
            ));
        }

        for sampler in &self.samplers {
            bindings.push_str(&format!(
                "{}",
                sampler.create_binding(group, bindings.len() as u32)
            ));
        }

        bindings
    }
}

pub fn vertex_input_struct() -> String {
    String::from(
        r#"struct VertexInput {
            @location(0) normal: vec3<f32>;
            @location(1) position: vec3<f32>;
            @location(2) uv: vec2<f32>;
        }"#,
    )
}

pub fn vertex_output_struct() -> String {
    String::from(
        r#"struct VertexOutput {
            @builtin(position) position: vec4<f32>;
            @location(0) normal: vec3<f32>;
            @location(1) uv: vec2<f32>;
        }"#,
    )
}
