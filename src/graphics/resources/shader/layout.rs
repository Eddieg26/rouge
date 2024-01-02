use crate::{
    ecs::resource::ResourceId,
    graphics::resources::{SamplerId, TextureId},
};
use itertools::Itertools;
use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    num::NonZeroU32,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

impl Display for ShaderAttribute {
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

impl From<naga::Binding> for ShaderAttribute {
    fn from(value: naga::Binding) -> Self {
        (&value).into()
    }
}

impl From<&naga::Binding> for ShaderAttribute {
    fn from(value: &naga::Binding) -> Self {
        match value {
            naga::Binding::BuiltIn(built_in) => match &built_in {
                naga::BuiltIn::VertexIndex => ShaderAttribute::VertexIndex,
                naga::BuiltIn::InstanceIndex => ShaderAttribute::InstanceIndex,
                naga::BuiltIn::Position { .. } => ShaderAttribute::Position,
                naga::BuiltIn::FrontFacing => ShaderAttribute::FrontFacing,
                naga::BuiltIn::FragDepth => ShaderAttribute::FragDepth,
                naga::BuiltIn::SampleIndex => ShaderAttribute::SampleIndex,
                naga::BuiltIn::SampleMask => ShaderAttribute::SampleMask,
                naga::BuiltIn::LocalInvocationId => ShaderAttribute::LocalInvocationId,
                naga::BuiltIn::LocalInvocationIndex => ShaderAttribute::LocalInvocationIndex,
                naga::BuiltIn::GlobalInvocationId => ShaderAttribute::GlobalInvocationId,
                naga::BuiltIn::WorkGroupId => ShaderAttribute::WorkgroupId,
                naga::BuiltIn::NumWorkGroups => ShaderAttribute::NumWorkgroups,
                _ => panic!("Unsupported built in"),
            },
            naga::Binding::Location { location, .. } => ShaderAttribute::Location(*location),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderVariable {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Bool,
    Array(Box<ShaderVariable>, usize),
    Struct(Vec<ShaderVariable>),
}

impl Display for ShaderVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderVariable::Float => write!(f, "Float"),
            ShaderVariable::Vec2 => write!(f, "Vec2"),
            ShaderVariable::Vec3 => write!(f, "Vec3"),
            ShaderVariable::Vec4 => write!(f, "Vec4"),
            ShaderVariable::Mat2 => write!(f, "Mat2"),
            ShaderVariable::Mat3 => write!(f, "Mat3"),
            ShaderVariable::Mat4 => write!(f, "Mat4"),
            ShaderVariable::Bool => write!(f, "Bool"),
            ShaderVariable::Array(ty, len) => write!(f, "Array({}, {})", ty, len),
            ShaderVariable::Struct(members) => write!(
                f,
                "Struct({})",
                members
                    .iter()
                    .map(|m| m.to_string())
                    .collect_vec()
                    .join(", ")
            ),
        }
    }
}

impl ShaderVariable {
    pub fn size(&self) -> usize {
        match self {
            ShaderVariable::Float => 4,
            ShaderVariable::Vec2 => 8,
            ShaderVariable::Vec3 => 12,
            ShaderVariable::Vec4 => 16,
            ShaderVariable::Mat2 => 16,
            ShaderVariable::Mat3 => 36,
            ShaderVariable::Mat4 => 64,
            ShaderVariable::Bool => 1,
            ShaderVariable::Array(ty, len) => ty.size() * len,
            ShaderVariable::Struct(members) => members.iter().map(|m| m.size()).sum(),
        }
    }

    pub fn aligned(&self) -> ShaderVariable {
        match self {
            ShaderVariable::Float => ShaderVariable::Vec2,
            ShaderVariable::Vec2 => ShaderVariable::Vec2,
            ShaderVariable::Vec3 => ShaderVariable::Vec4,
            ShaderVariable::Vec4 => ShaderVariable::Vec4,
            ShaderVariable::Mat2 => ShaderVariable::Mat2,
            ShaderVariable::Mat3 => ShaderVariable::Mat4,
            ShaderVariable::Mat4 => ShaderVariable::Mat4,
            ShaderVariable::Bool => ShaderVariable::Vec2,
            ShaderVariable::Array(ty, len) => ShaderVariable::Array(ty.aligned().into(), *len),
            ShaderVariable::Struct(members) => ShaderVariable::Struct(
                members
                    .iter()
                    .map(|m| m.aligned().into())
                    .collect::<Vec<ShaderVariable>>(),
            ),
        }
    }

    pub fn from_naga(
        value: &naga::TypeInner,
        types: &naga::UniqueArena<naga::Type>,
    ) -> ShaderVariable {
        match value {
            naga::TypeInner::Scalar { kind, .. } => match kind {
                naga::ScalarKind::Float => ShaderVariable::Float,
                naga::ScalarKind::Sint => ShaderVariable::Float,
                naga::ScalarKind::Uint => ShaderVariable::Float,
                naga::ScalarKind::Bool => ShaderVariable::Bool,
            },
            naga::TypeInner::Vector { size, kind, .. } => match kind {
                naga::ScalarKind::Float => match size {
                    naga::VectorSize::Bi => ShaderVariable::Vec2,
                    naga::VectorSize::Tri => ShaderVariable::Vec3,
                    naga::VectorSize::Quad => ShaderVariable::Vec4,
                },
                naga::ScalarKind::Sint => match size {
                    naga::VectorSize::Bi => ShaderVariable::Vec2,
                    naga::VectorSize::Tri => ShaderVariable::Vec3,
                    naga::VectorSize::Quad => ShaderVariable::Vec4,
                },
                naga::ScalarKind::Uint => match size {
                    naga::VectorSize::Bi => ShaderVariable::Vec2,
                    naga::VectorSize::Tri => ShaderVariable::Vec3,
                    naga::VectorSize::Quad => ShaderVariable::Vec4,
                },
                naga::ScalarKind::Bool => match size {
                    naga::VectorSize::Bi => ShaderVariable::Vec2,
                    naga::VectorSize::Tri => ShaderVariable::Vec3,
                    naga::VectorSize::Quad => ShaderVariable::Vec4,
                },
            },
            naga::TypeInner::Matrix { columns, rows, .. } => match (columns, rows) {
                (naga::VectorSize::Bi, naga::VectorSize::Bi) => ShaderVariable::Mat2,
                (naga::VectorSize::Tri, naga::VectorSize::Tri) => ShaderVariable::Mat3,
                (naga::VectorSize::Quad, naga::VectorSize::Quad) => ShaderVariable::Mat4,
                _ => panic!("Unsupported matrix size"),
            },
            naga::TypeInner::Array { base, size, .. } => {
                let ty = &types[*base];
                ShaderVariable::Array(
                    Box::new(ShaderVariable::from_naga(&ty.inner, types)),
                    match size {
                        naga::ArraySize::Constant(constant) => u32::from(*constant) as usize,
                        _ => panic!("Unsupported array size"),
                    },
                )
            }
            naga::TypeInner::Struct { members, .. } => {
                let mut variables = Vec::new();

                for member in members {
                    let ty = &types[member.ty];
                    variables.push(ShaderVariable::from_naga(&ty.inner, types));
                }

                ShaderVariable::Struct(variables)
            }
            _ => panic!("Unsupported type"),
        }
    }
}

impl Into<ResourceId> for &[ShaderVariable] {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ShaderDef {
    attribute: Option<ShaderAttribute>,
    variable: ShaderVariable,
}

impl Display for ShaderDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.attribute {
            Some(attribute) => write!(f, "{}: {}", attribute, self.variable),
            None => write!(f, "{}", self.variable),
        }
    }
}

impl ShaderDef {
    pub fn new(attribute: Option<ShaderAttribute>, variable: ShaderVariable) -> ShaderDef {
        ShaderDef {
            attribute,
            variable,
        }
    }

    pub fn attribute(&self) -> &Option<ShaderAttribute> {
        &self.attribute
    }

    pub fn variable(&self) -> &ShaderVariable {
        &self.variable
    }

    pub fn from_naga(
        binding: &Option<naga::Binding>,
        value: &naga::TypeInner,
        types: &naga::UniqueArena<naga::Type>,
    ) -> ShaderDef {
        let attribute = match binding {
            Some(binding) => Some(binding.into()),
            None => None,
        };
        ShaderDef::new(attribute, ShaderVariable::from_naga(value, types))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BufferLayout {
    attributes: Vec<ShaderVariable>,
}

impl Display for BufferLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({})",
            self.attributes
                .iter()
                .map(|a| a.to_string())
                .collect_vec()
                .join(", ")
        )
    }
}

impl BufferLayout {
    pub fn new(attributes: &[ShaderVariable]) -> BufferLayout {
        BufferLayout {
            attributes: attributes.to_vec(),
        }
    }

    pub fn attributes(&self) -> &[ShaderVariable] {
        &self.attributes
    }

    pub fn size(&self) -> usize {
        self.attributes.iter().map(|a| a.size()).sum()
    }

    pub fn offset(&self, index: usize) -> usize {
        self.attributes.iter().take(index).map(|a| a.size()).sum()
    }

    pub fn stride(&self) -> usize {
        self.attributes.iter().map(|a| a.size()).sum()
    }

    pub fn aligned(&self) -> BufferLayout {
        BufferLayout::new(&self.attributes.iter().map(|a| a.aligned()).collect_vec())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AccessMode {
    Read,
    Write,
}

impl Display for AccessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessMode::Read => write!(f, "read"),
            AccessMode::Write => write!(f, "write"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

impl Display for ShaderBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderBinding::UniformBuffer { layout, count } => {
                write!(
                    f,
                    "\tuniform buffer {{ count:{} }} layout{}",
                    count.map(|c| u32::from(c)).unwrap_or(0),
                    layout,
                )
            }
            ShaderBinding::StorageBuffer {
                layout,
                access,
                count,
            } => {
                write!(
                    f,
                    "\tuniform buffer {{ count:{}, access:{} }} layout{}",
                    count.map(|c| u32::from(c)).unwrap_or(0),
                    access,
                    layout,
                )
            }
            ShaderBinding::Texture2D => write!(f, "\ttexture 2d"),
            ShaderBinding::TextureCube => write!(f, "\ttexture cube"),
            ShaderBinding::Sampler => write!(f, "\tsampler"),
        }
    }
}

impl ShaderBinding {
    pub fn from_naga(
        ty: &naga::TypeInner,
        space: &naga::AddressSpace,
        types: &naga::UniqueArena<naga::Type>,
    ) -> ShaderBinding {
        match ty {
            naga::TypeInner::Struct { members, .. } => {
                let mut layout = Vec::new();

                for member in members {
                    let ty = &types[member.ty];
                    layout.push(ShaderVariable::from_naga(&ty.inner, types));
                }

                match space {
                    naga::AddressSpace::Uniform => ShaderBinding::UniformBuffer {
                        layout: BufferLayout::new(&layout),
                        count: None,
                    },
                    naga::AddressSpace::Storage { access } => ShaderBinding::StorageBuffer {
                        layout: BufferLayout::new(&layout),
                        access: match access.contains(naga::StorageAccess::STORE) {
                            true => AccessMode::Write,
                            false => AccessMode::Read,
                        },
                        count: None,
                    },
                    _ => panic!("Unsupported address space"),
                }
            }
            naga::TypeInner::Array { base, size, .. } => {
                let ty = &types[*base];
                let variable = ShaderVariable::from_naga(&ty.inner, types);

                match size {
                    naga::ArraySize::Constant(constant) => ShaderBinding::UniformBuffer {
                        layout: BufferLayout::new(&[variable]),
                        count: NonZeroU32::new(u32::from(*constant)),
                    },
                    _ => panic!("Unsupported array size"),
                }
            }
            naga::TypeInner::Image {
                dim,
                arrayed,
                class,
            } => match class {
                naga::ImageClass::Sampled { kind, .. } => match (arrayed, kind, dim) {
                    (false, naga::ScalarKind::Float, naga::ImageDimension::D2) => {
                        ShaderBinding::Texture2D
                    }
                    (false, naga::ScalarKind::Float, naga::ImageDimension::Cube) => {
                        ShaderBinding::TextureCube
                    }
                    (false, _, _) => panic!("Unsupported scalar"),
                    (true, _, _) => panic!("Unsupported arrayed image"),
                },
                _ => panic!("Unsupported image class"),
            },
            naga::TypeInner::Sampler { .. } => ShaderBinding::Sampler,
            _ => panic!("Unsupported type"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BindingId(u64);

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ShaderBindings {
    group: u32,
    bindings: Vec<ShaderBinding>,
}

impl Display for ShaderBindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "group:{}\n\tresources:\n{}\n",
            self.group,
            self.bindings
                .iter()
                .map(|b| b.to_string())
                .collect_vec()
                .join("\n\t")
        )
    }
}

impl ShaderBindings {
    pub fn new() -> ShaderBindings {
        ShaderBindings {
            group: 0,
            bindings: vec![],
        }
    }

    pub fn with_group(group: u32) -> ShaderBindings {
        ShaderBindings {
            group,
            bindings: vec![],
        }
    }

    pub fn from_slice(bindings: &[ShaderBinding]) -> ShaderBindings {
        ShaderBindings {
            group: 0,
            bindings: bindings.to_vec(),
        }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }

    pub fn add_binding(&mut self, binding: ShaderBinding) -> &mut Self {
        self.bindings.push(binding);

        self
    }

    pub fn insert_binding(&mut self, index: usize, binding: ShaderBinding) -> &mut Self {
        self.bindings.insert(index, binding);

        self
    }
}

impl From<Vec<ShaderBinding>> for ShaderBindings {
    fn from(bindings: Vec<ShaderBinding>) -> Self {
        ShaderBindings::from_slice(&bindings)
    }
}

impl Hash for ShaderBindings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for resource in &self.bindings {
            resource.hash(state);
        }
    }
}

impl Into<BindingId> for ShaderBindings {
    fn into(self) -> BindingId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        BindingId(hasher.finish())
    }
}

impl Into<BindingId> for &ShaderBindings {
    fn into(self) -> BindingId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        BindingId(hasher.finish())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShaderInput {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat2([f32; 4]),
    Mat3([f32; 9]),
    Mat4([f32; 16]),
    Bool(bool),
    Array(Box<ShaderInput>, usize),
}

impl Display for ShaderInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderInput::Float(v) => write!(f, "{}", v),
            ShaderInput::Vec2(v) => write!(f, "{}, {}", v[0], v[1]),
            ShaderInput::Vec3(v) => write!(f, "{}, {}, {}", v[0], v[1], v[2]),
            ShaderInput::Vec4(v) => write!(f, "{}, {}, {}, {}", v[0], v[1], v[2], v[3]),
            ShaderInput::Mat2(v) => write!(f, "{}, {},\n{}, {}", v[0], v[1], v[2], v[3],),
            ShaderInput::Mat3(v) => write!(
                f,
                "{}, {}, {},\n{}, {}, {},\n{}, {}, {}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
            ),
            ShaderInput::Mat4(v) => write!(
                f,
                "{}, {}, {}, {},\n{}, {}, {}, {},\n{}, {}, {}, {},\n{}, {}, {}, {}",
                v[0],
                v[1],
                v[2],
                v[3],
                v[4],
                v[5],
                v[6],
                v[7],
                v[8],
                v[9],
                v[10],
                v[11],
                v[12],
                v[13],
                v[14],
                v[15],
            ),
            ShaderInput::Bool(v) => write!(f, "{}", v),
            ShaderInput::Array(ty, len) => write!(f, "ty:{} len:{}", ty, len),
        }
    }
}

impl Hash for ShaderInput {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let var: ShaderVariable = self.clone().into();
        var.hash(state);
    }
}

impl Into<ShaderVariable> for ShaderInput {
    fn into(self) -> ShaderVariable {
        match self {
            ShaderInput::Float(_) => ShaderVariable::Float,
            ShaderInput::Vec2(_) => ShaderVariable::Vec2,
            ShaderInput::Vec3(_) => ShaderVariable::Vec3,
            ShaderInput::Vec4(_) => ShaderVariable::Vec4,
            ShaderInput::Mat2(_) => ShaderVariable::Mat2,
            ShaderInput::Mat3(_) => ShaderVariable::Mat3,
            ShaderInput::Mat4(_) => ShaderVariable::Mat4,
            ShaderInput::Bool(_) => ShaderVariable::Bool,
            ShaderInput::Array(ty, len) => {
                ShaderVariable::Array(Box::new(ty.as_ref().clone().into()), len)
            }
        }
    }
}

impl From<ShaderVariable> for ShaderInput {
    fn from(value: ShaderVariable) -> Self {
        match value {
            ShaderVariable::Float => ShaderInput::Float(0.0),
            ShaderVariable::Vec2 => ShaderInput::Vec2([0.0; 2]),
            ShaderVariable::Vec3 => ShaderInput::Vec3([0.0; 3]),
            ShaderVariable::Vec4 => ShaderInput::Vec4([0.0; 4]),
            ShaderVariable::Mat2 => ShaderInput::Mat2([0.0; 4]),
            ShaderVariable::Mat3 => ShaderInput::Mat3([0.0; 9]),
            ShaderVariable::Mat4 => ShaderInput::Mat4([0.0; 16]),
            ShaderVariable::Bool => ShaderInput::Bool(false),
            ShaderVariable::Array(ty, len) => {
                ShaderInput::Array(Box::new(ty.as_ref().clone().into()), len)
            }
            _ => panic!("Invalid variable type"),
        }
    }
}

impl ShaderInput {
    pub fn size(&self) -> usize {
        match self {
            ShaderInput::Float(_) => 4,
            ShaderInput::Vec2(_) => 8,
            ShaderInput::Vec3(_) => 12,
            ShaderInput::Vec4(_) => 16,
            ShaderInput::Mat2(_) => 16,
            ShaderInput::Mat3(_) => 36,
            ShaderInput::Mat4(_) => 64,
            ShaderInput::Bool(_) => 1,
            ShaderInput::Array(ty, len) => ty.size() * len,
        }
    }

    pub fn align(&self) -> ShaderInput {
        match self {
            ShaderInput::Float(v) => ShaderInput::Vec2([*v; 2]),
            ShaderInput::Vec2(v) => ShaderInput::Vec2(*v),
            ShaderInput::Vec3(v) => ShaderInput::Vec4([v[0], v[1], v[2], 0.0]),
            ShaderInput::Vec4(v) => ShaderInput::Vec4(*v),
            ShaderInput::Mat2(v) => ShaderInput::Mat2(*v),
            ShaderInput::Mat3(v) => ShaderInput::Mat4([
                v[0], v[1], v[2], 0.0, v[3], v[4], v[5], 0.0, v[6], v[7], v[8], 0.0, 0.0, 0.0, 0.0,
                0.0,
            ]),
            ShaderInput::Mat4(v) => ShaderInput::Mat4(*v),
            ShaderInput::Bool(v) => ShaderInput::Vec2([*v as u32 as f32, 0.0]),
            ShaderInput::Array(ty, len) => ShaderInput::Array(ty.align().into(), *len),
        }
    }

    pub fn aligned(inputs: &[ShaderInput]) -> Vec<f32> {
        let mut buffer = Vec::new();

        for input in inputs {
            match input.align() {
                ShaderInput::Vec2(v) => buffer.extend_from_slice(&v),
                ShaderInput::Vec3(v) => buffer.extend_from_slice(&v),
                ShaderInput::Vec4(v) => buffer.extend_from_slice(&v),
                ShaderInput::Mat2(v) => buffer.extend_from_slice(&v),
                ShaderInput::Mat4(v) => buffer.extend_from_slice(&v),
                ShaderInput::Array(ty, len) => {
                    for _ in 0..len {
                        buffer.extend_from_slice(&ShaderInput::aligned(
                            &[ty.as_ref().clone()].to_vec(),
                        ));
                    }
                }
                _ => panic!("Invalid input type"),
            }
        }

        buffer
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BufferType {
    Uniform,
    Storage(AccessMode),
}

impl Display for BufferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferType::Uniform => write!(f, "uniform"),
            BufferType::Storage(access) => write!(f, "storage {}", access),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BufferInfo {
    inputs: Vec<ShaderInput>,
    ty: BufferType,
}

impl BufferInfo {
    pub fn new(ty: BufferType) -> BufferInfo {
        BufferInfo {
            inputs: Vec::new(),
            ty,
        }
    }

    pub fn inputs(&self) -> &[ShaderInput] {
        &self.inputs
    }

    pub fn ty(&self) -> &BufferType {
        &self.ty
    }

    pub fn add_input(mut self, input: ShaderInput) -> Self {
        self.inputs.push(input);

        self
    }

    pub fn aligned(&self) -> BufferInfo {
        let mut info = BufferInfo::new(self.ty.clone());

        for input in &self.inputs {
            info = info.add_input(input.align());
        }

        info
    }

    pub fn buffer(&self) -> Vec<f32> {
        ShaderInput::aligned(&self.inputs)
    }
}

impl Display for BufferInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ty:{}\n inputs:{}]",
            self.ty,
            self.inputs
                .iter()
                .map(|i| i.to_string())
                .collect_vec()
                .join(", ")
        )
    }
}

impl Into<BufferLayout> for BufferInfo {
    fn into(self) -> BufferLayout {
        BufferLayout::new(&self.inputs.iter().map(|i| i.clone().into()).collect_vec())
    }
}

impl Into<ResourceId> for BufferInfo {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inputs.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

impl Into<ResourceId> for &BufferInfo {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inputs.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

#[derive(Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShaderResource {
    Buffer(BufferInfo),
    Texture2D(TextureId),
    TextureCube(TextureId),
    Sampler(SamplerId),
}

impl Display for ShaderResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderResource::Buffer(info) => write!(f, "buffer {}", info),
            ShaderResource::Texture2D(id) => write!(f, "texture 2d {}", id),
            ShaderResource::TextureCube(id) => write!(f, "texture cube {}", id),
            ShaderResource::Sampler(id) => write!(f, "sampler {}", id),
        }
    }
}

impl Into<ShaderBinding> for ShaderResource {
    fn into(self) -> ShaderBinding {
        match self {
            ShaderResource::Buffer(info) => ShaderBinding::UniformBuffer {
                layout: info.into(),
                count: None,
            },
            ShaderResource::Texture2D(_) => ShaderBinding::Texture2D,
            ShaderResource::TextureCube(_) => ShaderBinding::TextureCube,
            ShaderResource::Sampler(_) => ShaderBinding::Sampler,
        }
    }
}

impl Into<u32> for ShaderResource {
    fn into(self) -> u32 {
        (&self).into()
    }
}

impl Into<u32> for &ShaderResource {
    fn into(self) -> u32 {
        match self {
            ShaderResource::Buffer(_) => 0,
            ShaderResource::Texture2D(_) => 1,
            ShaderResource::TextureCube(_) => 2,
            ShaderResource::Sampler(_) => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShaderResources(Vec<ShaderResource>);

impl ShaderResources {
    pub fn new() -> ShaderResources {
        ShaderResources(Vec::new())
    }

    pub fn inner(&self) -> &[ShaderResource] {
        &self.0
    }

    pub fn add_resource(mut self, resource: ShaderResource) -> Self {
        self.0.push(resource);

        self
    }
}

impl Display for ShaderResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.0
                .iter()
                .map(|r| r.to_string())
                .collect_vec()
                .join(", ")
        )
    }
}

impl Hash for ShaderResources {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for resource in &self.0 {
            resource.hash(state);
        }
    }
}

impl Into<ResourceId> for ShaderResources {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

impl Into<ResourceId> for &ShaderResources {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

impl From<Vec<ShaderResource>> for ShaderResources {
    fn from(resources: Vec<ShaderResource>) -> Self {
        ShaderResources(resources)
    }
}
