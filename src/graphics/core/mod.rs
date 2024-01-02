pub mod device;
pub mod surface;
pub mod vertex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderModel {
    Lit,
    Unlit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
