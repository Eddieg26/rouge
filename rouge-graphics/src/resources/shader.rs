use rouge_asset::Asset;

use crate::core::ty::shader::{IntoBindGroupLayout, ShaderBindGroup, ShaderVariable};

pub struct ShaderMeta {
    entry: String,
    bindings: Vec<ShaderBindGroup>,
    inputs: Vec<ShaderVariable>,
    outputs: Vec<ShaderVariable>,
}

impl ShaderMeta {
    pub fn new(
        entry: String,
        bindings: Vec<ShaderBindGroup>,
        inputs: Vec<ShaderVariable>,
        outputs: Vec<ShaderVariable>,
    ) -> Self {
        Self {
            entry,
            bindings,
            inputs,
            outputs,
        }
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn bindings(&self) -> &[ShaderBindGroup] {
        &self.bindings
    }

    pub fn inputs(&self) -> &[ShaderVariable] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[ShaderVariable] {
        &self.outputs
    }

    pub fn create_layouts(&self, device: &wgpu::Device) -> Vec<wgpu::BindGroupLayout> {
        self.bindings
            .iter()
            .map(|group| group.into_bind_group_layout(device))
            .collect()
    }
}

pub struct Shader {
    module: wgpu::ShaderModule,
    vertex: Option<ShaderMeta>,
    fragment: Option<ShaderMeta>,
    compute: Option<ShaderMeta>,
}

impl Shader {
    pub fn new(
        module: wgpu::ShaderModule,
        vertex: Option<ShaderMeta>,
        fragment: Option<ShaderMeta>,
        compute: Option<ShaderMeta>,
    ) -> Self {
        Self {
            module,
            vertex,
            fragment,
            compute,
        }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn vertex(&self) -> Option<&ShaderMeta> {
        self.vertex.as_ref()
    }

    pub fn fragment(&self) -> Option<&ShaderMeta> {
        self.fragment.as_ref()
    }

    pub fn compute(&self) -> Option<&ShaderMeta> {
        self.compute.as_ref()
    }

    pub fn vertex_mut(&mut self) -> Option<&mut ShaderMeta> {
        self.vertex.as_mut()
    }
}

impl Asset for Shader {}
