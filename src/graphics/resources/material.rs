use std::hash::{Hash, Hasher};

use itertools::Itertools;

use super::{ShaderId, TextureId};

#[derive(Debug, Clone, Copy)]
pub enum ShaderInput {
    Texture(TextureId),
    Color(wgpu::Color),
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
pub struct LitShaderInputs {
    pub normal: ShaderInput,
    pub specular: ShaderInput,
    pub metallic: ShaderInput,
    pub roughness: ShaderInput,
    pub emission: ShaderInput,
}

impl LitShaderInputs {
    pub fn new() -> LitShaderInputs {
        LitShaderInputs {
            normal: ShaderInput::Scalar(0.0),
            specular: ShaderInput::Scalar(0.0),
            metallic: ShaderInput::Scalar(0.0),
            roughness: ShaderInput::Scalar(0.0),
            emission: ShaderInput::Scalar(0.0),
        }
    }

    pub fn with_normal(mut self, normal: ShaderInput) -> LitShaderInputs {
        self.normal = normal;
        self
    }

    pub fn with_specular(mut self, specular: ShaderInput) -> LitShaderInputs {
        self.specular = specular;
        self
    }

    pub fn with_metallic(mut self, metallic: ShaderInput) -> LitShaderInputs {
        self.metallic = metallic;
        self
    }

    pub fn with_roughness(mut self, roughness: ShaderInput) -> LitShaderInputs {
        self.roughness = roughness;
        self
    }

    pub fn with_emission(mut self, emission: ShaderInput) -> LitShaderInputs {
        self.emission = emission;
        self
    }

    pub fn model(&self) -> ShaderModel {
        ShaderModel::Lit {
            normal: self.normal,
            specular: self.specular,
            metallic: self.metallic,
            roughness: self.roughness,
            emission: self.emission,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ShaderModel {
    Lit {
        normal: ShaderInput,
        specular: ShaderInput,
        metallic: ShaderInput,
        roughness: ShaderInput,
        emission: ShaderInput,
    },
    Unlit,
}

impl Hash for ShaderModel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShaderModel::Lit { .. } => 0.hash(state),
            ShaderModel::Unlit => 1.hash(state),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Material {
    shader_model: ShaderModel,
    blend_mode: BlendMode,
    color: ShaderInput,
}

impl Material {
    pub fn new(shader_model: ShaderModel) -> Material {
        Material {
            shader_model,
            blend_mode: BlendMode::Opaque,
            color: ShaderInput::Color(wgpu::Color::WHITE),
        }
    }

    pub fn shader_model(&self) -> &ShaderModel {
        &self.shader_model
    }

    pub fn color(&self) -> &ShaderInput {
        &self.color
    }

    pub fn blend_mode(&self) -> &BlendMode {
        &self.blend_mode
    }

    pub fn is_lit(&self) -> bool {
        matches!(self.shader_model, ShaderModel::Lit { .. })
    }

    pub fn is_opaque(&self) -> bool {
        matches!(self.blend_mode, BlendMode::Opaque)
    }

    pub fn set_shader_model(&mut self, shader_model: ShaderModel) {
        self.shader_model = shader_model;
    }

    pub fn set_color(&mut self, color: ShaderInput) {
        self.color = color;
    }

    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.blend_mode = blend_mode;
    }

    pub(super) fn shader_id(&self) -> ShaderId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);

        ShaderId::new(hasher.finish())
    }

    pub fn textures<'a>(&'a self) -> Vec<&'a TextureId> {
        let mut textures = vec![&self.color];
        match self.shader_model {
            ShaderModel::Lit {
                ref normal,
                ref specular,
                ref metallic,
                ref roughness,
                ref emission,
            } => {
                textures.append(&mut vec![normal, specular, metallic, roughness, emission]);
            }
            ShaderModel::Unlit => {}
        }

        match self.blend_mode {
            BlendMode::Transparent(ref input) => textures.push(input),
            BlendMode::Opaque => {}
        }

        textures
            .iter()
            .filter_map(|input| match input {
                ShaderInput::Texture(id) => Some(id),
                _ => None,
            })
            .collect_vec()
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
