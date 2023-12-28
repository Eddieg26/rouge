use super::{
    shader::graph::attribute::{BufferProperty, PropertyBlock, TextureProperty},
    ShaderId, TextureId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Lit,
    Unlit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl BlendMode {
    pub fn color_target_state(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
        match self {
            BlendMode::Opaque => wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            },
            BlendMode::Transparent => wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            },
        }
    }
}

pub struct Material {
    shader_id: ShaderId,
    properties: PropertyBlock,
}

impl Material {
    pub fn new(shader_id: ShaderId) -> Material {
        Material {
            shader_id,
            properties: PropertyBlock::new(),
        }
    }

    pub fn inputs(&self) -> &[BufferProperty] {
        &self.properties.inputs()
    }

    pub fn textures(&self) -> &[TextureProperty] {
        &self.properties.textures()
    }

    pub fn shader_id(&self) -> &ShaderId {
        &self.shader_id
    }

    pub fn set_properties(&mut self, properties: PropertyBlock) {
        self.properties = properties;
    }

    pub fn set_input(&mut self, input: BufferProperty) {
        self.properties.set_input(input);
    }

    pub fn set_texture(
        &mut self,
        name: &str,
        id: TextureId,
        dimension: wgpu::TextureViewDimension,
    ) {
        self.properties.set_texture(name, &id, dimension);
    }
}
