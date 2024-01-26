use super::field::ShaderField;
use std::hash::Hash;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

impl Hash for ShaderAttribute {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShaderAttribute::VertexIndex => {
                state.write_u8(0);
            }
            ShaderAttribute::InstanceIndex => {
                state.write_u8(1);
            }
            ShaderAttribute::Position => {
                state.write_u8(2);
            }
            ShaderAttribute::FrontFacing => {
                state.write_u8(3);
            }
            ShaderAttribute::FragDepth => {
                state.write_u8(4);
            }
            ShaderAttribute::SampleIndex => {
                state.write_u8(5);
            }
            ShaderAttribute::SampleMask => {
                state.write_u8(6);
            }
            ShaderAttribute::LocalInvocationId => {
                state.write_u8(7);
            }
            ShaderAttribute::LocalInvocationIndex => {
                state.write_u8(8);
            }
            ShaderAttribute::GlobalInvocationId => {
                state.write_u8(9);
            }
            ShaderAttribute::WorkgroupId => {
                state.write_u8(10);
            }
            ShaderAttribute::NumWorkgroups => {
                state.write_u8(11);
            }
            ShaderAttribute::Location(location) => {
                state.write_u8(12);
                state.write_u32(*location);
            }
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
        match self.attribute() {
            Some(attribute) => write!(f, "{} {}", attribute, self.field()),
            None => write!(f, "{}", self.field()),
        }
    }
}
