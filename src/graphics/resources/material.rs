use super::TextureId;

#[derive(Debug, Clone, Copy)]
pub enum ShaderInput {
    Texture(TextureId),
    Color([f32; 4]),
    Scalar(f32),
}

impl Into<u32> for ShaderInput {
    fn into(self) -> u32 {
        match self {
            ShaderInput::Texture(_) => 0,
            ShaderInput::Color(_) => 1,
            ShaderInput::Scalar(_) => 2,
        }
    }
}
impl Eq for ShaderInput {}

impl PartialEq for ShaderInput {
    fn eq(&self, other: &Self) -> bool {
        let self_idx: u32 = (*self).into();
        let other_idx: u32 = (*other).into();
        self_idx == other_idx
    }
}

impl Ord for ShaderInput {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_idx: u32 = (*self).into();
        let other_idx: u32 = (*other).into();
        self_idx.cmp(&other_idx)
    }
}

impl PartialOrd for ShaderInput {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_idx: u32 = (*self).into();
        let other_idx: u32 = (*other).into();
        self_idx.partial_cmp(&other_idx)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlendMode {
    Opaque,
    Transparent(ShaderInput),
}

impl std::hash::Hash for BlendMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            BlendMode::Opaque => 0.hash(state),
            BlendMode::Transparent(_) => 1.hash(state),
        }
    }
}

impl std::hash::Hash for ShaderInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShaderInput::Texture(_) => 0.hash(state),
            ShaderInput::Color(_) => 1.hash(state),
            ShaderInput::Scalar(_) => 2.hash(state),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct LitShaderModel {
    pub color: ShaderInput,
    pub normal: ShaderInput,
    pub specular: ShaderInput,
    pub metallic: ShaderInput,
    pub roughness: ShaderInput,
    pub emission: ShaderInput,
    pub blend_mode: BlendMode,
}

impl LitShaderModel {
    pub fn new(color: ShaderInput) -> LitShaderModel {
        LitShaderModel {
            color,
            normal: ShaderInput::Scalar(0.0),
            specular: ShaderInput::Scalar(0.0),
            metallic: ShaderInput::Scalar(0.0),
            roughness: ShaderInput::Scalar(0.0),
            emission: ShaderInput::Scalar(0.0),
            blend_mode: BlendMode::Opaque,
        }
    }

    pub fn with_normal(mut self, normal: ShaderInput) -> LitShaderModel {
        self.normal = normal;
        self
    }

    pub fn with_specular(mut self, specular: ShaderInput) -> LitShaderModel {
        self.specular = specular;
        self
    }

    pub fn with_metallic(mut self, metallic: ShaderInput) -> LitShaderModel {
        self.metallic = metallic;
        self
    }

    pub fn with_roughness(mut self, roughness: ShaderInput) -> LitShaderModel {
        self.roughness = roughness;
        self
    }

    pub fn with_emission(mut self, emission: ShaderInput) -> LitShaderModel {
        self.emission = emission;
        self
    }

    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> LitShaderModel {
        self.blend_mode = blend_mode;
        self
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct UnlitShaderModel {
    pub color: ShaderInput,
    pub blend_mode: BlendMode,
}

impl UnlitShaderModel {
    pub fn new() -> UnlitShaderModel {
        UnlitShaderModel {
            color: ShaderInput::Color([1.0, 1.0, 1.0, 1.0]),
            blend_mode: BlendMode::Opaque,
        }
    }

    pub fn with_color(mut self, color: ShaderInput) -> UnlitShaderModel {
        self.color = color;
        self
    }

    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> UnlitShaderModel {
        self.blend_mode = blend_mode;
        self
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub enum ShaderModel {
    Lit(LitShaderModel),
    Unlit(UnlitShaderModel),
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Material {
    shader_model: ShaderModel,
}

impl Material {
    pub fn new(shader_model: ShaderModel) -> Material {
        Material { shader_model }
    }

    pub fn shader_model(&self) -> &ShaderModel {
        &self.shader_model
    }

    pub fn textures(&self) -> Vec<TextureId> {
        match self.shader_model {
            ShaderModel::Lit(ref model) => {
                let mut textures = vec![
                    model.color,
                    model.normal,
                    model.specular,
                    model.metallic,
                    model.roughness,
                    model.emission,
                ];
                match model.blend_mode {
                    BlendMode::Opaque => {}
                    BlendMode::Transparent(input) => textures.push(input),
                }
                textures
                    .into_iter()
                    .filter_map(|input| match input {
                        ShaderInput::Texture(id) => Some(id),
                        _ => None,
                    })
                    .collect()
            }
            ShaderModel::Unlit(ref model) => {
                let mut textures = vec![model.color];
                match model.blend_mode {
                    BlendMode::Opaque => {}
                    BlendMode::Transparent(input) => textures.push(input),
                }

                textures
                    .into_iter()
                    .filter_map(|input| match input {
                        ShaderInput::Texture(id) => Some(id),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            }
        }
    }
}

pub struct InputNames;

impl InputNames {
    pub const COLOR: &'static str = "color";
    pub const NORMAL: &'static str = "normal";
    pub const SPECULAR: &'static str = "specular";
    pub const METALLIC: &'static str = "metallic";
    pub const ROUGHNESS: &'static str = "roughness";
    pub const EMISSION: &'static str = "emission";
    pub const OPACITY: &'static str = "opacity";
}
