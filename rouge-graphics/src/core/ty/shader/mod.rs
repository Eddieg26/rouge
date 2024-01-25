use self::field::{ShaderField, ShaderInput};
use crate::core::ResourceId;
use std::hash::Hash;
use std::num::NonZeroU32;

pub mod field;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderAttribute {
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
    Location(u32),
}

impl std::fmt::Display for ShaderAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderAttribute::VertexIndex => write!(f, "VertexIndex"),
            ShaderAttribute::InstanceIndex => write!(f, "InstanceIndex"),
            ShaderAttribute::Position => write!(f, "Position"),
            ShaderAttribute::FrontFacing => write!(f, "FrontFacing"),
            ShaderAttribute::FragDepth => write!(f, "FragDepth"),
            ShaderAttribute::SampleIndex => write!(f, "SampleIndex"),
            ShaderAttribute::SampleMask => write!(f, "SampleMask"),
            ShaderAttribute::LocalInvocationId => write!(f, "LocalInvocationId"),
            ShaderAttribute::LocalInvocationIndex => write!(f, "LocalInvocationIndex"),
            ShaderAttribute::GlobalInvocationId => write!(f, "GlobalInvocationId"),
            ShaderAttribute::WorkgroupId => write!(f, "WorkgroupId"),
            ShaderAttribute::NumWorkgroups => write!(f, "NumWorkgroups"),
            ShaderAttribute::Location(location) => write!(f, "Location({})", location),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderVariable {
    attribute: Option<ShaderAttribute>,
    field: ShaderField,
}

impl ShaderVariable {
    pub fn new(attribute: Option<ShaderAttribute>, field: ShaderField) -> Self {
        Self { attribute, field }
    }

    pub fn attribute(&self) -> Option<ShaderAttribute> {
        self.attribute
    }

    pub fn field(&self) -> &ShaderField {
        &self.field
    }
}

impl std::fmt::Display for ShaderVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.attribute {
            Some(attribute) => write!(f, "{} {}", attribute, self.field),
            None => write!(f, "{}", self.field),
        }
    }
}

pub struct BufferLayout {
    variables: Vec<ShaderVariable>,
}

impl std::fmt::Display for BufferLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({})",
            self.variables
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl BufferLayout {
    pub fn new(variables: Vec<ShaderVariable>) -> Self {
        Self { variables }
    }

    pub fn variables(&self) -> &[ShaderVariable] {
        &self.variables
    }

    pub fn size(&self) -> u32 {
        let mut size = 0;
        for variable in &self.variables {
            size += variable.field.size();
        }
        size
    }

    pub fn aligned(&self) -> BufferLayout {
        let mut variables = Vec::new();
        for variable in &self.variables {
            variables.push(ShaderVariable::new(
                variable.attribute,
                variable.field.aligned(),
            ));
        }
        BufferLayout::new(variables)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

pub enum ShaderBinding {
    UniformBuffer {
        layout: BufferLayout,
        count: Option<NonZeroU32>,
    },
    StorageBuffer {
        layout: BufferLayout,
        access: AccessMode,
        count: Option<NonZeroU32>,
    },
    Texture2D,
    TextureCube,
    Sampler,
}

impl std::fmt::Display for ShaderBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderBinding::UniformBuffer { layout, count } => {
                write!(
                    f,
                    "uniform_buffer({}, {})",
                    layout,
                    count.unwrap_or(NonZeroU32::new(1).unwrap())
                )
            }
            ShaderBinding::StorageBuffer {
                layout,
                access,
                count,
            } => write!(
                f,
                "storage_buffer({}, {}, {})",
                layout,
                access,
                count.unwrap_or(NonZeroU32::new(1).unwrap())
            ),
            ShaderBinding::Texture2D => write!(f, "texture_2d"),
            ShaderBinding::TextureCube => write!(f, "texture_cube"),
            ShaderBinding::Sampler => write!(f, "sampler"),
        }
    }
}

pub struct ShaderBindGroup {
    group: u32,
    bindings: Vec<ShaderBinding>,
}

impl ShaderBindGroup {
    pub fn new(group: u32, bindings: Vec<ShaderBinding>) -> Self {
        Self { group, bindings }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }
}

impl std::fmt::Display for ShaderBindGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "group:{}\n\tresources:\n{}\n",
            self.group,
            self.bindings
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join("\n\t")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug, PartialEq)]
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
        BufferLayout::new(variables)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShaderResource {
    Buffer(ShaderBuffer),
    Texture2D(ResourceId),
    TextureCube(ResourceId),
    Sampler(ResourceId),
}

impl std::fmt::Display for ShaderResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderResource::Buffer(buffer) => write!(f, "{}", buffer),
            ShaderResource::Texture2D(id) => write!(f, "texture_2d({})", id),
            ShaderResource::TextureCube(id) => write!(f, "texture_cube({})", id),
            ShaderResource::Sampler(id) => write!(f, "sampler({})", id),
        }
    }
}

pub struct ShaderResourceGroup {
    group: u32,
    resources: Vec<ShaderResource>,
}

impl ShaderResourceGroup {
    pub fn new(group: u32, resources: Vec<ShaderResource>) -> Self {
        Self { group, resources }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn resources(&self) -> &[ShaderResource] {
        &self.resources
    }
}

impl std::fmt::Display for ShaderResourceGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[group:{}\n resources:{}]",
            self.group,
            self.resources
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
