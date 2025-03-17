use crate::device::RenderDevice;
use asset::Asset;
use std::{borrow::Cow, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl Into<wgpu::naga::ShaderStage> for ShaderStage {
    fn into(self) -> wgpu::naga::ShaderStage {
        match self {
            Self::Vertex => wgpu::naga::ShaderStage::Vertex,
            Self::Fragment => wgpu::naga::ShaderStage::Fragment,
            Self::Compute => wgpu::naga::ShaderStage::Compute,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Asset)]
pub enum ShaderSource {
    Spirv {
        data: Cow<'static, [u32]>,
    },
    Glsl {
        data: Cow<'static, str>,
        stage: ShaderStage,
    },
    Wgsl {
        data: Cow<'static, str>,
    },
}

pub struct Shader(Arc<wgpu::ShaderModule>);
impl Shader {
    pub fn new(device: &RenderDevice, source: ShaderSource) -> Self {
        let module = match source {
            ShaderSource::Spirv { data } => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::SpirV(Cow::Borrowed(&data)),
                })
            }
            ShaderSource::Glsl { data, stage } => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: data,
                        stage: stage.into(),
                        defines: Default::default(),
                    },
                })
            }
            ShaderSource::Wgsl { data } => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&data)),
                })
            }
        };
        Self(Arc::new(module))
    }
}

impl From<wgpu::ShaderModule> for Shader {
    fn from(shader: wgpu::ShaderModule) -> Self {
        Self(Arc::new(shader))
    }
}

impl std::ops::Deref for Shader {
    type Target = wgpu::ShaderModule;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<wgpu::ShaderModule> for Shader {
    fn as_ref(&self) -> &wgpu::ShaderModule {
        &self.0
    }
}
