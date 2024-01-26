use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShaderField {
    Float32,
    Int32,
    Uint32,
    Float64,
    Int64,
    Uint64,
    Boolean,
    Vec2f32,
    Vec3f32,
    Vec4f32,
    Vec2i32,
    Vec3i32,
    Vec4i32,
    Vec2u32,
    Vec3u32,
    Vec4u32,
    Vec2f64,
    Vec3f64,
    Vec4f64,
    Vec2i64,
    Vec3i64,
    Vec4i64,
    Vec2u64,
    Vec3u64,
    Vec4u64,
    Mat2f32,
    Mat3f32,
    Mat4f32,
    Mat2i32,
    Mat3i32,
    Mat4i32,
    Mat2u32,
    Mat3u32,
    Mat4u32,
    Mat2f64,
    Mat3f64,
    Mat4f64,
    Mat2i64,
    Mat3i64,
    Mat4i64,
    Mat2u64,
    Mat3u64,
    Mat4u64,
    Array(Box<ShaderField>, u32),
    Struct(Vec<ShaderField>),
}

impl ShaderField {
    pub fn size(&self) -> u32 {
        match self {
            ShaderField::Float32 => 4,
            ShaderField::Int32 => 4,
            ShaderField::Uint32 => 4,
            ShaderField::Float64 => 8,
            ShaderField::Int64 => 8,
            ShaderField::Uint64 => 8,
            ShaderField::Boolean => 1,
            ShaderField::Vec2f32 => 8,
            ShaderField::Vec3f32 => 12,
            ShaderField::Vec4f32 => 16,
            ShaderField::Vec2i32 => 8,
            ShaderField::Vec3i32 => 12,
            ShaderField::Vec4i32 => 16,
            ShaderField::Vec2u32 => 8,
            ShaderField::Vec3u32 => 12,
            ShaderField::Vec4u32 => 16,
            ShaderField::Vec2f64 => 16,
            ShaderField::Vec3f64 => 24,
            ShaderField::Vec4f64 => 32,
            ShaderField::Vec2i64 => 16,
            ShaderField::Vec3i64 => 24,
            ShaderField::Vec4i64 => 32,
            ShaderField::Vec2u64 => 16,
            ShaderField::Vec3u64 => 24,
            ShaderField::Vec4u64 => 32,
            ShaderField::Mat2f32 => 16,
            ShaderField::Mat3f32 => 36,
            ShaderField::Mat4f32 => 64,
            ShaderField::Mat2i32 => 16,
            ShaderField::Mat3i32 => 36,
            ShaderField::Mat4i32 => 64,
            ShaderField::Mat2u32 => 16,
            ShaderField::Mat3u32 => 36,
            ShaderField::Mat4u32 => 64,
            ShaderField::Mat2f64 => 32,
            ShaderField::Mat3f64 => 72,
            ShaderField::Mat4f64 => 128,
            ShaderField::Mat2i64 => 32,
            ShaderField::Mat3i64 => 72,
            ShaderField::Mat4i64 => 128,
            ShaderField::Mat2u64 => 32,
            ShaderField::Mat3u64 => 72,
            ShaderField::Mat4u64 => 128,
            ShaderField::Array(field, length) => field.size() * length,
            ShaderField::Struct(fields) => fields.iter().map(|f| f.size()).sum(),
        }
    }

    pub fn aligned(&self) -> Self {
        match self {
            ShaderField::Float32 => ShaderField::Vec2f32,
            ShaderField::Int32 => ShaderField::Vec2i32,
            ShaderField::Uint32 => ShaderField::Vec2u32,
            ShaderField::Float64 => ShaderField::Vec2f64,
            ShaderField::Int64 => ShaderField::Vec2i64,
            ShaderField::Uint64 => ShaderField::Vec2u64,
            ShaderField::Boolean => ShaderField::Boolean,
            ShaderField::Vec3f32 => ShaderField::Vec4f32,
            ShaderField::Vec3i32 => ShaderField::Vec4i32,
            ShaderField::Vec3u32 => ShaderField::Vec4u32,
            ShaderField::Vec3f64 => ShaderField::Vec4f64,
            ShaderField::Vec3i64 => ShaderField::Vec4i64,
            ShaderField::Vec3u64 => ShaderField::Vec4u64,
            ShaderField::Mat3f32 => ShaderField::Mat4f32,
            ShaderField::Mat3i32 => ShaderField::Mat4i32,
            ShaderField::Mat3u32 => ShaderField::Mat4u32,
            ShaderField::Mat3f64 => ShaderField::Mat4f64,
            ShaderField::Mat3i64 => ShaderField::Mat4i64,
            ShaderField::Mat3u64 => ShaderField::Mat4u64,
            ShaderField::Array(field, length) => {
                let aligned = field.aligned();
                let size = aligned.size();
                let length = (length + size - 1) / size;
                ShaderField::Array(Box::new(aligned), length)
            }
            ShaderField::Struct(fields) => {
                let mut aligned_fields = Vec::new();
                let mut offset = 0;
                for field in fields {
                    let aligned = field.aligned();
                    let size = aligned.size();
                    let padding = ((offset + size - 1) / size * size - offset) / 4;
                    if padding > 0 {
                        aligned_fields.push(ShaderField::Array(
                            Box::new(ShaderField::Uint32),
                            padding / 4,
                        ));
                        offset += padding;
                    }
                    aligned_fields.push(aligned);
                    offset += size;
                }
                ShaderField::Struct(aligned_fields)
            }
            _ => self.clone(),
        }
    }
}

impl std::fmt::Display for ShaderField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderField::Float32 => write!(f, "f32"),
            ShaderField::Int32 => write!(f, "i32"),
            ShaderField::Uint32 => write!(f, "u32"),
            ShaderField::Float64 => write!(f, "f64"),
            ShaderField::Int64 => write!(f, "i64"),
            ShaderField::Uint64 => write!(f, "u64"),
            ShaderField::Boolean => write!(f, "bool"),
            ShaderField::Vec2f32 => write!(f, "vec2<f32>"),
            ShaderField::Vec3f32 => write!(f, "vec3<f32>"),
            ShaderField::Vec4f32 => write!(f, "vec4<f32>"),
            ShaderField::Vec2i32 => write!(f, "vec2<i32>"),
            ShaderField::Vec3i32 => write!(f, "vec3<i32>"),
            ShaderField::Vec4i32 => write!(f, "vec4<i32>"),
            ShaderField::Vec2u32 => write!(f, "vec2<u32>"),
            ShaderField::Vec3u32 => write!(f, "vec3<u32>"),
            ShaderField::Vec4u32 => write!(f, "vec4<u32>"),
            ShaderField::Vec2f64 => write!(f, "vec2<f64>"),
            ShaderField::Vec3f64 => write!(f, "vec3<f64>"),
            ShaderField::Vec4f64 => write!(f, "vec4<f64>"),
            ShaderField::Vec2i64 => write!(f, "vec2<i64>"),
            ShaderField::Vec3i64 => write!(f, "vec3<i64>"),
            ShaderField::Vec4i64 => write!(f, "vec4<i64>"),
            ShaderField::Vec2u64 => write!(f, "vec2<u64>"),
            ShaderField::Vec3u64 => write!(f, "vec3<u64>"),
            ShaderField::Vec4u64 => write!(f, "vec4<u64>"),
            ShaderField::Mat2f32 => write!(f, "mat2x2<f32>"),
            ShaderField::Mat3f32 => write!(f, "mat3x3<f32>"),
            ShaderField::Mat4f32 => write!(f, "mat4x4<f32>"),
            ShaderField::Mat2i32 => write!(f, "mat2x2<i32>"),
            ShaderField::Mat3i32 => write!(f, "mat3x3<i32>"),
            ShaderField::Mat4i32 => write!(f, "mat4x4<i32>"),
            ShaderField::Mat2u32 => write!(f, "mat2x2<u32>"),
            ShaderField::Mat3u32 => write!(f, "mat3x3<u32>"),
            ShaderField::Mat4u32 => write!(f, "mat4x4<u32>"),
            ShaderField::Mat2f64 => write!(f, "mat2x2<f64>"),
            ShaderField::Mat3f64 => write!(f, "mat3x3<f64>"),
            ShaderField::Mat4f64 => write!(f, "mat4x4<f64>"),
            ShaderField::Mat2i64 => write!(f, "mat2x2<i64>"),
            ShaderField::Mat3i64 => write!(f, "mat3x3<i64>"),
            ShaderField::Mat4i64 => write!(f, "mat4x4<i64>"),
            ShaderField::Mat2u64 => write!(f, "mat2x2<u64>"),
            ShaderField::Mat3u64 => write!(f, "mat3x3<u64>"),
            ShaderField::Mat4u64 => write!(f, "mat4x4<u64>"),
            ShaderField::Array(field, length) => write!(f, "[{}; {}]", field, length),
            ShaderField::Struct(fields) => {
                write!(f, "{{\n")?;
                for field in fields {
                    write!(f, "\t{},\n", field)?;
                }
                write!(f, "}}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShaderInput {
    Float32(f32),
    Int32(i32),
    Uint32(u32),
    Float64(f64),
    Int64(i64),
    Uint64(u64),
    Boolean(bool),
    Vec2f32([f32; 2]),
    Vec3f32([f32; 3]),
    Vec4f32([f32; 4]),
    Vec2i32([i32; 2]),
    Vec3i32([i32; 3]),
    Vec4i32([i32; 4]),
    Vec2u32([u32; 2]),
    Vec3u32([u32; 3]),
    Vec4u32([u32; 4]),
    Vec2f64([f64; 2]),
    Vec3f64([f64; 3]),
    Vec4f64([f64; 4]),
    Vec2i64([i64; 2]),
    Vec3i64([i64; 3]),
    Vec4i64([i64; 4]),
    Vec2u64([u64; 2]),
    Vec3u64([u64; 3]),
    Vec4u64([u64; 4]),
    Mat2f32([[f32; 2]; 2]),
    Mat3f32([[f32; 3]; 3]),
    Mat4f32([[f32; 4]; 4]),
    Mat2i32([[i32; 2]; 2]),
    Mat3i32([[i32; 3]; 3]),
    Mat4i32([[i32; 4]; 4]),
    Mat2u32([[u32; 2]; 2]),
    Mat3u32([[u32; 3]; 3]),
    Mat4u32([[u32; 4]; 4]),
    Mat2f64([[f64; 2]; 2]),
    Mat3f64([[f64; 3]; 3]),
    Mat4f64([[f64; 4]; 4]),
    Mat2i64([[i64; 2]; 2]),
    Mat3i64([[i64; 3]; 3]),
    Mat4i64([[i64; 4]; 4]),
    Mat2u64([[u64; 2]; 2]),
    Mat3u64([[u64; 3]; 3]),
    Mat4u64([[u64; 4]; 4]),
    Array(Vec<ShaderInput>),
    Struct(Vec<ShaderInput>),
}

impl ShaderInput {
    pub fn size(&self) -> u32 {
        match self {
            ShaderInput::Float32(_) => 4,
            ShaderInput::Int32(_) => 4,
            ShaderInput::Uint32(_) => 4,
            ShaderInput::Float64(_) => 8,
            ShaderInput::Int64(_) => 8,
            ShaderInput::Uint64(_) => 8,
            ShaderInput::Boolean(_) => 1,
            ShaderInput::Vec2f32(_) => 8,
            ShaderInput::Vec3f32(_) => 12,
            ShaderInput::Vec4f32(_) => 16,
            ShaderInput::Vec2i32(_) => 8,
            ShaderInput::Vec3i32(_) => 12,
            ShaderInput::Vec4i32(_) => 16,
            ShaderInput::Vec2u32(_) => 8,
            ShaderInput::Vec3u32(_) => 12,
            ShaderInput::Vec4u32(_) => 16,
            ShaderInput::Vec2f64(_) => 16,
            ShaderInput::Vec3f64(_) => 24,
            ShaderInput::Vec4f64(_) => 32,
            ShaderInput::Vec2i64(_) => 16,
            ShaderInput::Vec3i64(_) => 24,
            ShaderInput::Vec4i64(_) => 32,
            ShaderInput::Vec2u64(_) => 16,
            ShaderInput::Vec3u64(_) => 24,
            ShaderInput::Vec4u64(_) => 32,
            ShaderInput::Mat2f32(_) => 16,
            ShaderInput::Mat3f32(_) => 36,
            ShaderInput::Mat4f32(_) => 64,
            ShaderInput::Mat2i32(_) => 16,
            ShaderInput::Mat3i32(_) => 36,
            ShaderInput::Mat4i32(_) => 64,
            ShaderInput::Mat2u32(_) => 16,
            ShaderInput::Mat3u32(_) => 36,
            ShaderInput::Mat4u32(_) => 64,
            ShaderInput::Mat2f64(_) => 32,
            ShaderInput::Mat3f64(_) => 72,
            ShaderInput::Mat4f64(_) => 128,
            ShaderInput::Mat2i64(_) => 32,
            ShaderInput::Mat3i64(_) => 72,
            ShaderInput::Mat4i64(_) => 128,
            ShaderInput::Mat2u64(_) => 32,
            ShaderInput::Mat3u64(_) => 72,
            ShaderInput::Mat4u64(_) => 128,
            ShaderInput::Array(inputs) => inputs.iter().map(|i| i.size()).sum(),
            ShaderInput::Struct(inputs) => inputs.iter().map(|i| i.size()).sum(),
        }
    }

    pub fn aligned(&self) -> Self {
        match self {
            ShaderInput::Float32(_) => ShaderInput::Vec2f32([0.0; 2]),
            ShaderInput::Int32(_) => ShaderInput::Vec2i32([0; 2]),
            ShaderInput::Uint32(_) => ShaderInput::Vec2u32([0; 2]),
            ShaderInput::Float64(_) => ShaderInput::Vec2f64([0.0; 2]),
            ShaderInput::Int64(_) => ShaderInput::Vec2i64([0; 2]),
            ShaderInput::Uint64(_) => ShaderInput::Vec2u64([0; 2]),
            ShaderInput::Boolean(_) => ShaderInput::Boolean(false),
            ShaderInput::Vec3f32(_) => ShaderInput::Vec4f32([0.0; 4]),
            ShaderInput::Vec3i32(_) => ShaderInput::Vec4i32([0; 4]),
            ShaderInput::Vec3u32(_) => ShaderInput::Vec4u32([0; 4]),
            ShaderInput::Vec3f64(_) => ShaderInput::Vec4f64([0.0; 4]),
            ShaderInput::Vec3i64(_) => ShaderInput::Vec4i64([0; 4]),
            ShaderInput::Vec3u64(_) => ShaderInput::Vec4u64([0; 4]),
            ShaderInput::Mat3f32(_) => ShaderInput::Mat4f32([[0.0; 4]; 4]),
            ShaderInput::Mat3i32(_) => ShaderInput::Mat4i32([[0; 4]; 4]),
            ShaderInput::Mat3u32(_) => ShaderInput::Mat4u32([[0; 4]; 4]),
            ShaderInput::Mat3f64(_) => ShaderInput::Mat4f64([[0.0; 4]; 4]),
            ShaderInput::Mat3i64(_) => ShaderInput::Mat4i64([[0; 4]; 4]),
            ShaderInput::Mat3u64(_) => ShaderInput::Mat4u64([[0; 4]; 4]),
            ShaderInput::Array(inputs) => {
                let mut aligned_inputs = Vec::new();
                for input in inputs {
                    aligned_inputs.push(input.aligned());
                }
                ShaderInput::Array(aligned_inputs)
            }
            ShaderInput::Struct(inputs) => {
                let mut aligned_inputs = Vec::new();
                let mut offset = 0;
                for input in inputs {
                    let aligned = input.aligned();
                    let size = aligned.size();
                    let padding = ((offset + size - 1) / size * size - offset) / 4;
                    if padding > 0 {
                        aligned_inputs.push(ShaderInput::Array(vec![
                            ShaderInput::Uint32(0);
                            padding as usize
                        ]));
                        offset += padding;
                    }
                    aligned_inputs.push(aligned);
                    offset += size;
                }
                ShaderInput::Struct(aligned_inputs)
            }
            _ => self.clone(),
        }
    }
}

impl Into<ShaderField> for ShaderInput {
    fn into(self) -> ShaderField {
        match self {
            ShaderInput::Float32(_) => ShaderField::Float32,
            ShaderInput::Int32(_) => ShaderField::Int32,
            ShaderInput::Uint32(_) => ShaderField::Uint32,
            ShaderInput::Float64(_) => ShaderField::Float64,
            ShaderInput::Int64(_) => ShaderField::Int64,
            ShaderInput::Uint64(_) => ShaderField::Uint64,
            ShaderInput::Boolean(_) => ShaderField::Boolean,
            ShaderInput::Vec2f32(_) => ShaderField::Vec2f32,
            ShaderInput::Vec3f32(_) => ShaderField::Vec3f32,
            ShaderInput::Vec4f32(_) => ShaderField::Vec4f32,
            ShaderInput::Vec2i32(_) => ShaderField::Vec2i32,
            ShaderInput::Vec3i32(_) => ShaderField::Vec3i32,
            ShaderInput::Vec4i32(_) => ShaderField::Vec4i32,
            ShaderInput::Vec2u32(_) => ShaderField::Vec2u32,
            ShaderInput::Vec3u32(_) => ShaderField::Vec3u32,
            ShaderInput::Vec4u32(_) => ShaderField::Vec4u32,
            ShaderInput::Vec2f64(_) => ShaderField::Vec2f64,
            ShaderInput::Vec3f64(_) => ShaderField::Vec3f64,
            ShaderInput::Vec4f64(_) => ShaderField::Vec4f64,
            ShaderInput::Vec2i64(_) => ShaderField::Vec2i64,
            ShaderInput::Vec3i64(_) => ShaderField::Vec3i64,
            ShaderInput::Vec4i64(_) => ShaderField::Vec4i64,
            ShaderInput::Vec2u64(_) => ShaderField::Vec2u64,
            ShaderInput::Vec3u64(_) => ShaderField::Vec3u64,
            ShaderInput::Vec4u64(_) => ShaderField::Vec4u64,
            ShaderInput::Mat2f32(_) => ShaderField::Mat2f32,
            ShaderInput::Mat3f32(_) => ShaderField::Mat3f32,
            ShaderInput::Mat4f32(_) => ShaderField::Mat4f32,
            ShaderInput::Mat2i32(_) => ShaderField::Mat2i32,
            ShaderInput::Mat3i32(_) => ShaderField::Mat3i32,
            ShaderInput::Mat4i32(_) => ShaderField::Mat4i32,
            ShaderInput::Mat2u32(_) => ShaderField::Mat2u32,
            ShaderInput::Mat3u32(_) => ShaderField::Mat3u32,
            ShaderInput::Mat4u32(_) => ShaderField::Mat4u32,
            ShaderInput::Mat2f64(_) => ShaderField::Mat2f64,
            ShaderInput::Mat3f64(_) => ShaderField::Mat3f64,
            ShaderInput::Mat4f64(_) => ShaderField::Mat4f64,
            ShaderInput::Mat2i64(_) => ShaderField::Mat2i64,
            ShaderInput::Mat3i64(_) => ShaderField::Mat3i64,
            ShaderInput::Mat4i64(_) => ShaderField::Mat4i64,
            ShaderInput::Mat2u64(_) => ShaderField::Mat2u64,
            ShaderInput::Mat3u64(_) => ShaderField::Mat3u64,
            ShaderInput::Mat4u64(_) => ShaderField::Mat4u64,
            ShaderInput::Array(inputs) => {
                let field = inputs.first().unwrap_or(&ShaderInput::Float32(0.0));
                ShaderField::Array(Box::new(field.clone().into()), inputs.len() as u32)
            }
            ShaderInput::Struct(inputs) => {
                let mut fields = Vec::new();
                for input in inputs {
                    fields.push(input.into());
                }
                ShaderField::Struct(fields)
            }
        }
    }
}

impl From<ShaderField> for ShaderInput {
    fn from(field: ShaderField) -> Self {
        match field {
            ShaderField::Float32 => ShaderInput::Float32(0.0),
            ShaderField::Int32 => ShaderInput::Int32(0),
            ShaderField::Uint32 => ShaderInput::Uint32(0),
            ShaderField::Float64 => ShaderInput::Float64(0.0),
            ShaderField::Int64 => ShaderInput::Int64(0),
            ShaderField::Uint64 => ShaderInput::Uint64(0),
            ShaderField::Boolean => ShaderInput::Boolean(false),
            ShaderField::Vec2f32 => ShaderInput::Vec2f32([0.0; 2]),
            ShaderField::Vec3f32 => ShaderInput::Vec3f32([0.0; 3]),
            ShaderField::Vec4f32 => ShaderInput::Vec4f32([0.0; 4]),
            ShaderField::Vec2i32 => ShaderInput::Vec2i32([0; 2]),
            ShaderField::Vec3i32 => ShaderInput::Vec3i32([0; 3]),
            ShaderField::Vec4i32 => ShaderInput::Vec4i32([0; 4]),
            ShaderField::Vec2u32 => ShaderInput::Vec2u32([0; 2]),
            ShaderField::Vec3u32 => ShaderInput::Vec3u32([0; 3]),
            ShaderField::Vec4u32 => ShaderInput::Vec4u32([0; 4]),
            ShaderField::Vec2f64 => ShaderInput::Vec2f64([0.0; 2]),
            ShaderField::Vec3f64 => ShaderInput::Vec3f64([0.0; 3]),
            ShaderField::Vec4f64 => ShaderInput::Vec4f64([0.0; 4]),
            ShaderField::Vec2i64 => ShaderInput::Vec2i64([0; 2]),
            ShaderField::Vec3i64 => ShaderInput::Vec3i64([0; 3]),
            ShaderField::Vec4i64 => ShaderInput::Vec4i64([0; 4]),
            ShaderField::Vec2u64 => ShaderInput::Vec2u64([0; 2]),
            ShaderField::Vec3u64 => ShaderInput::Vec3u64([0; 3]),
            ShaderField::Vec4u64 => ShaderInput::Vec4u64([0; 4]),
            ShaderField::Mat2f32 => ShaderInput::Mat2f32([[0.0; 2]; 2]),
            ShaderField::Mat3f32 => ShaderInput::Mat3f32([[0.0; 3]; 3]),
            ShaderField::Mat4f32 => ShaderInput::Mat4f32([[0.0; 4]; 4]),
            ShaderField::Mat2i32 => ShaderInput::Mat2i32([[0; 2]; 2]),
            ShaderField::Mat3i32 => ShaderInput::Mat3i32([[0; 3]; 3]),
            ShaderField::Mat4i32 => ShaderInput::Mat4i32([[0; 4]; 4]),
            ShaderField::Mat2u32 => ShaderInput::Mat2u32([[0; 2]; 2]),
            ShaderField::Mat3u32 => ShaderInput::Mat3u32([[0; 3]; 3]),
            ShaderField::Mat4u32 => ShaderInput::Mat4u32([[0; 4]; 4]),
            ShaderField::Mat2f64 => ShaderInput::Mat2f64([[0.0; 2]; 2]),
            ShaderField::Mat3f64 => ShaderInput::Mat3f64([[0.0; 3]; 3]),
            ShaderField::Mat4f64 => ShaderInput::Mat4f64([[0.0; 4]; 4]),
            ShaderField::Mat2i64 => ShaderInput::Mat2i64([[0; 2]; 2]),
            ShaderField::Mat3i64 => ShaderInput::Mat3i64([[0; 3]; 3]),
            ShaderField::Mat4i64 => ShaderInput::Mat4i64([[0; 4]; 4]),
            ShaderField::Mat2u64 => ShaderInput::Mat2u64([[0; 2]; 2]),
            ShaderField::Mat3u64 => ShaderInput::Mat3u64([[0; 3]; 3]),
            ShaderField::Mat4u64 => ShaderInput::Mat4u64([[0; 4]; 4]),
            ShaderField::Array(_, length) => {
                ShaderInput::Array(Vec::with_capacity(length as usize))
            }
            ShaderField::Struct(fields) => {
                let mut inputs = Vec::new();
                for field in fields {
                    inputs.push(field.into());
                }
                ShaderInput::Struct(inputs)
            }
        }
    }
}

impl std::fmt::Display for ShaderInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderInput::Float32(value) => write!(f, "{}", value),
            ShaderInput::Int32(value) => write!(f, "{}", value),
            ShaderInput::Uint32(value) => write!(f, "{}", value),
            ShaderInput::Float64(value) => write!(f, "{}", value),
            ShaderInput::Int64(value) => write!(f, "{}", value),
            ShaderInput::Uint64(value) => write!(f, "{}", value),
            ShaderInput::Boolean(value) => write!(f, "{}", value),
            ShaderInput::Vec2f32(value) => write!(f, "vec2<f32>({}, {})", value[0], value[1]),
            ShaderInput::Vec3f32(value) => {
                write!(f, "vec3<f32>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4f32(value) => write!(
                f,
                "vec4<f32>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Vec2i32(value) => write!(f, "vec2<i32>({}, {})", value[0], value[1]),
            ShaderInput::Vec3i32(value) => {
                write!(f, "vec3<i32>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4i32(value) => write!(
                f,
                "vec4<i32>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Vec2u32(value) => write!(f, "vec2<u32>({}, {})", value[0], value[1]),
            ShaderInput::Vec3u32(value) => {
                write!(f, "vec3<u32>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4u32(value) => write!(
                f,
                "vec4<u32>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Vec2f64(value) => write!(f, "vec2<f64>({}, {})", value[0], value[1]),
            ShaderInput::Vec3f64(value) => {
                write!(f, "vec3<f64>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4f64(value) => write!(
                f,
                "vec4<f64>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Vec2i64(value) => write!(f, "vec2<i64>({}, {})", value[0], value[1]),
            ShaderInput::Vec3i64(value) => {
                write!(f, "vec3<i64>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4i64(value) => write!(
                f,
                "vec4<i64>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Vec2u64(value) => write!(f, "vec2<u64>({}, {})", value[0], value[1]),
            ShaderInput::Vec3u64(value) => {
                write!(f, "vec3<u64>({}, {}, {})", value[0], value[1], value[2])
            }
            ShaderInput::Vec4u64(value) => write!(
                f,
                "vec4<u64>({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            ShaderInput::Mat2f32(value) => write!(
                f,
                "mat2x2<f32>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3f32(value) => write!(
                f,
                "mat3x3<f32>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4f32(value) => write!(
                f,
                "mat4x4<f32>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Mat2i32(value) => write!(
                f,
                "mat2x2<i32>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3i32(value) => write!(
                f,
                "mat3x3<i32>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4i32(value) => write!(
                f,
                "mat4x4<i32>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Mat2u32(value) => write!(
                f,
                "mat2x2<u32>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3u32(value) => write!(
                f,
                "mat3x3<u32>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4u32(value) => write!(
                f,
                "mat4x4<u32>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Mat2f64(value) => write!(
                f,
                "mat2x2<f64>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3f64(value) => write!(
                f,
                "mat3x3<f64>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4f64(value) => write!(
                f,
                "mat4x4<f64>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Mat2i64(value) => write!(
                f,
                "mat2x2<i64>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3i64(value) => write!(
                f,
                "mat3x3<i64>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4i64(value) => write!(
                f,
                "mat4x4<i64>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Mat2u64(value) => write!(
                f,
                "mat2x2<u64>({}, {}, {}, {})",
                value[0][0], value[0][1], value[1][0], value[1][1]
            ),
            ShaderInput::Mat3u64(value) => write!(
                f,
                "mat3x3<u64>({}, {}, {}, {}, {}, {}, {}, {}, {})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[1][0],
                value[1][1],
                value[1][2],
                value[2][0],
                value[2][1],
                value[2][2]
            ),
            ShaderInput::Mat4u64(value) => write!(
                f,
                "mat4x4<u64>({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, \
                 {10}, {11}, {12}, {13}, {14}, {15})",
                value[0][0],
                value[0][1],
                value[0][2],
                value[0][3],
                value[1][0],
                value[1][1],
                value[1][2],
                value[1][3],
                value[2][0],
                value[2][1],
                value[2][2],
                value[2][3],
                value[3][0],
                value[3][1],
                value[3][2],
                value[3][3]
            ),
            ShaderInput::Array(inputs) => {
                write!(f, "[")?;
                for input in inputs {
                    write!(f, "{}, ", input)?;
                }
                write!(f, "]")
            }
            ShaderInput::Struct(inputs) => {
                write!(f, "{{\n")?;
                for input in inputs {
                    write!(f, "\t{},\n", input)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Hash for ShaderInput {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let field: ShaderField = self.clone().into();

        field.hash(state);
    }
}
