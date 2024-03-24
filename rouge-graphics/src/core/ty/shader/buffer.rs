use super::{attribute::ShaderVariable, field::ShaderInput};
use rouge_core::ResourceId;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferLayout {
    variables: Vec<ShaderVariable>,
    ty: BufferType,
}

impl std::fmt::Display for BufferLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ty:{}\n variables:{}] size:{}",
            self.ty,
            self.variables
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.size()
        )
    }
}

impl BufferLayout {
    pub fn new(variables: Vec<ShaderVariable>, ty: BufferType) -> Self {
        Self { variables, ty }
    }

    pub fn ty(&self) -> &BufferType {
        &self.ty
    }

    pub fn variables(&self) -> &[ShaderVariable] {
        &self.variables
    }

    pub fn size(&self) -> u32 {
        let mut size = 0;
        for variable in &self.variables {
            size += variable.field().size();
        }
        size
    }

    pub fn aligned(&self) -> BufferLayout {
        let mut variables = Vec::new();
        for variable in &self.variables {
            variables.push(ShaderVariable::new(
                variable.attribute(),
                variable.field().aligned(),
            ));
        }
        BufferLayout::new(variables, self.ty)
    }
}

impl Into<ResourceId> for BufferLayout {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessMode {
    Read,
    Write,
}

impl std::fmt::Display for AccessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessMode::Read => write!(f, "read"),
            AccessMode::Write => write!(f, "write"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    Uniform,
    Storage(AccessMode),
}

impl std::fmt::Display for BufferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferType::Uniform => write!(f, "uniform"),
            BufferType::Storage(access) => write!(f, "storage {}", access),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ShaderBuffer {
    inputs: Vec<ShaderInput>,
    ty: BufferType,
}

impl ShaderBuffer {
    pub fn new(inputs: Vec<ShaderInput>, ty: BufferType) -> Self {
        Self { inputs, ty }
    }

    pub fn inputs(&self) -> &[ShaderInput] {
        &self.inputs
    }

    pub fn ty(&self) -> &BufferType {
        &self.ty
    }

    pub fn aligned(&self) -> ShaderBuffer {
        let mut inputs = Vec::new();
        for input in &self.inputs {
            inputs.push(input.aligned());
        }
        ShaderBuffer::new(inputs, self.ty.clone())
    }
}

impl std::fmt::Display for ShaderBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ty:{}\n inputs:{}]",
            self.ty,
            self.inputs
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl Into<BufferLayout> for ShaderBuffer {
    fn into(self) -> BufferLayout {
        let mut variables = Vec::new();
        for input in self.inputs {
            variables.push(ShaderVariable::new(None, input.into()));
        }
        BufferLayout::new(variables, self.ty)
    }
}

impl Into<ResourceId> for ShaderBuffer {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}
