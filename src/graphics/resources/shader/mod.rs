use std::fmt::Display;

use self::layout::{ShaderBindings, ShaderDef};
use crate::asset::{Asset, AssetMetadata};

pub mod layout;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ShaderMeta {
    entry: String,
    bindings: Vec<ShaderBindings>,
    inputs: Vec<ShaderDef>,
    outputs: Vec<ShaderDef>,
}

impl ShaderMeta {
    pub fn new(
        entry: &str,
        bindings: Vec<ShaderBindings>,
        inputs: Vec<ShaderDef>,
        outputs: Vec<ShaderDef>,
    ) -> ShaderMeta {
        ShaderMeta {
            entry: entry.to_string(),
            bindings,
            inputs,
            outputs,
        }
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn bindings(&self) -> &[ShaderBindings] {
        &self.bindings
    }

    pub fn inputs(&self) -> &[ShaderDef] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[ShaderDef] {
        &self.outputs
    }
}

impl Display for ShaderMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Entry: {}", self.entry)?;
        writeln!(f, "Bindings:")?;
        for binding in self.bindings.iter() {
            writeln!(f, "\t{}", binding)?;
        }
        writeln!(f, "Inputs:")?;
        for input in self.inputs.iter() {
            writeln!(f, "\t{}", input)?;
        }
        writeln!(f, "Outputs:")?;
        for output in self.outputs.iter() {
            writeln!(f, "\t{}", output)?;
        }
        Ok(())
    }
}

impl AssetMetadata for ShaderMeta {}

pub struct Shader {
    module: wgpu::ShaderModule,
    meta: ShaderMeta,
}

impl Shader {
    pub fn new(module: wgpu::ShaderModule, meta: ShaderMeta) -> Shader {
        Shader { module, meta }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn meta(&self) -> &ShaderMeta {
        &self.meta
    }

    pub fn validate(&self, other: &Shader) -> bool {
        self.meta.outputs() == other.meta().inputs()
    }
}

impl Asset for Shader {}
