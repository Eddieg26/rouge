use crate::graphics::resources::{
    material::{BlendMode, ShaderModel},
    GpuResources, TextureId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attribute {
    Float,
    Vec2,
    Vec2u,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Color,
    Bool,
    Texture2D,
    Texture3D,
    Texture2DArray,
    CubeMap,
}

impl Attribute {
    pub fn cast(&self, name: &str, other: &Attribute) -> String {
        if self == other {
            return name.to_string();
        }

        match (self, other) {
            (Attribute::Float, Attribute::Vec2) => format!("vec2({})", name),
            (Attribute::Float, Attribute::Vec3) => format!("vec3({})", name),
            (Attribute::Float, Attribute::Vec4) => format!("vec4({})", name),
            (Attribute::Float, Attribute::Color) => format!("vec4({}, 1.0)", name),
            (Attribute::Float, Attribute::Mat2) => format!("mat2({})", name),
            (Attribute::Float, Attribute::Mat3) => {
                format!("mat3({}, vec3(0.0, 0.0, 1.0))", name)
            }
            (Attribute::Float, Attribute::Mat4) => {
                format!("mat4({}, vec4(0.0, 0.0, 1.0, 0.0))", name)
            }
            (Attribute::Vec2, Attribute::Float) => format!("{}.x", name),
            (Attribute::Vec2, Attribute::Vec3) => format!("vec3({}, 0.0)", name),
            (Attribute::Vec2, Attribute::Vec4) => format!("vec4({}, 0.0, 0.0)", name),
            (Attribute::Vec2, Attribute::Color) => format!("vec4({}, 0.0, 0.0)", name),
            (Attribute::Vec2, Attribute::Mat2) => format!("mat2({})", name),
            (Attribute::Vec2, Attribute::Mat3) => {
                format!("mat3({}, vec3(0.0, 0.0, 1.0))", name)
            }
            (Attribute::Vec2, Attribute::Mat4) => {
                format!("mat4({}, vec4(0.0, 0.0, 1.0, 0.0))", name)
            }
            (Attribute::Vec2u, Attribute::Float) => format!("{}.x", name),
            (Attribute::Vec2u, Attribute::Vec2) => format!("vec2({}.x, {}.y)", name, name),
            (Attribute::Vec2u, Attribute::Vec3) => format!("vec3({}.x, {}.y, 0.0)", name, name),
            (Attribute::Vec2u, Attribute::Vec4) => {
                format!("vec4({}.x, {}.y, 0.0, 0.0)", name, name)
            }
            (Attribute::Vec2u, Attribute::Color) => {
                format!("vec4({}.x, {}.y, 0.0, 0.0)", name, name)
            }
            (Attribute::Vec2u, Attribute::Mat2) => format!("mat2({})", name),
            (Attribute::Vec2u, Attribute::Mat3) => {
                format!("mat3({}, vec3(0.0, 0.0, 1.0))", name)
            }
            (Attribute::Vec2u, Attribute::Mat4) => {
                format!("mat4({}, vec4(0.0, 0.0, 1.0, 0.0))", name)
            }
            (Attribute::Vec3, Attribute::Float) => format!("{}.x", name),
            (Attribute::Vec3, Attribute::Vec2) => format!("{}.xy", name),
            (Attribute::Vec3, Attribute::Vec4) => format!("vec4({}, 1.0)", name),
            (Attribute::Vec3, Attribute::Color) => format!("vec4({}, 1.0)", name),
            (Attribute::Vec3, Attribute::Mat3) => format!("mat3({})", name),
            (Attribute::Vec3, Attribute::Mat4) => {
                format!("mat4({}, vec4(0.0, 0.0, 1.0, 0.0))", name)
            }
            (Attribute::Vec4, Attribute::Float) => format!("{}.x", name),
            (Attribute::Vec4, Attribute::Vec2) => format!("{}.xy", name),
            (Attribute::Vec4, Attribute::Vec3) => format!("{}.xyz", name),
            (Attribute::Vec4, Attribute::Color) => format!("{}", name),
            (Attribute::Vec4, Attribute::Mat4) => format!("mat4({})", name),
            (Attribute::Mat2, Attribute::Float) => format!("{}.x", name),
            (Attribute::Mat2, Attribute::Vec2) => format!("{}.x", name),
            (Attribute::Mat2, Attribute::Vec3) => format!("{}.xy", name),
            (Attribute::Mat2, Attribute::Vec4) => format!("{}.xyz", name),
            (Attribute::Mat2, Attribute::Color) => format!("{}.xyz", name),
            (Attribute::Mat3, Attribute::Float) => format!("{}.x", name),
            (Attribute::Mat3, Attribute::Vec2) => format!("{}.x", name),
            (Attribute::Mat3, Attribute::Vec3) => format!("{}.xyz", name),
            (Attribute::Mat3, Attribute::Vec4) => format!("{}.xyz", name),
            (Attribute::Mat3, Attribute::Color) => format!("{}.xyz", name),
            (Attribute::Mat4, Attribute::Float) => format!("{}.x", name),
            (Attribute::Mat4, Attribute::Vec2) => format!("{}.x", name),
            (Attribute::Mat4, Attribute::Vec3) => format!("{}.xyz", name),
            (Attribute::Mat4, Attribute::Vec4) => format!("{}.xyz", name),
            (Attribute::Mat4, Attribute::Color) => format!("{}.xyz", name),
            (Attribute::Color, Attribute::Float) => format!("{}.r", name),
            (Attribute::Color, Attribute::Vec2) => format!("{}.rg", name),
            (Attribute::Color, Attribute::Vec3) => format!("{}.rgb", name),
            (Attribute::Color, Attribute::Vec4) => format!("{}.rgba", name),
            _ => panic!("Cannot cast {:?} to {:?}", self, other),
        }
    }

    pub fn definition(&self, name: &str, prefix: &str) -> String {
        match self {
            Attribute::Float => format!("{} {} : f32;\n", prefix, name),
            Attribute::Vec2 => format!("{} {} : vec2<f32>;\n", prefix, name),
            Attribute::Vec2u => format!("{} {} : vec2<u32>;\n", prefix, name),
            Attribute::Vec3 => format!("{} {} : vec3<f32>;\n", prefix, name),
            Attribute::Vec4 => format!("{} {} : vec4<f32>;\n", prefix, name),
            Attribute::Mat2 => format!("{} {} : mat2x2<f32>;\n", prefix, name),
            Attribute::Mat3 => format!("{} {} : mat3x3<f32>;\n", prefix, name),
            Attribute::Mat4 => format!("{} {} : mat4x4<f32>;\n", prefix, name),
            Attribute::Color => format!("{} {} : vec4<f32>;\n", prefix, name),
            Attribute::Bool => format!("{} {} : bool;\n", prefix, name),
            Attribute::Texture2D => {
                format!("{} var {} : texture_2d<f32>;\n", prefix, name)
            }
            Attribute::Texture3D => {
                format!("{} var {} : texture_3d<f32>;\n", prefix, name)
            }
            Attribute::Texture2DArray => {
                format!("{} var {} : texture_2d_array<f32>;\n", prefix, name)
            }
            Attribute::CubeMap => {
                format!("{} var {} : texture_cube<f32>;\n", prefix, name)
            }
        }
    }

    pub fn is_texture(&self) -> bool {
        match self {
            Attribute::Texture2D
            | Attribute::Texture3D
            | Attribute::Texture2DArray
            | Attribute::CubeMap => true,
            _ => false,
        }
    }

    pub fn split_textures(attributes: &[Attribute]) -> (Vec<Attribute>, Vec<Attribute>) {
        let mut textures = Vec::new();
        let mut attributes = attributes.to_vec();

        attributes.retain(|a| {
            if a.is_texture() {
                textures.push(*a);
                false
            } else {
                true
            }
        });

        (attributes, textures)
    }

    pub fn size(&self) -> u32 {
        match self {
            Attribute::Float => 1,
            Attribute::Vec2 => 2,
            Attribute::Vec2u => 2,
            Attribute::Vec3 => 3,
            Attribute::Vec4 => 4,
            Attribute::Mat2 => 4,
            Attribute::Mat3 => 9,
            Attribute::Mat4 => 16,
            Attribute::Color => 4,
            Attribute::Bool => 1,
            Attribute::Texture2D => u32::MAX,
            Attribute::Texture3D => u32::MAX,
            Attribute::Texture2DArray => u32::MAX,
            Attribute::CubeMap => u32::MAX,
        }
    }

    pub fn add_padding(attributes: &[Attribute]) -> Vec<Attribute> {
        let mut list = attributes.to_vec();

        for attribute in attributes.iter() {
            match attribute {
                Attribute::Float => list.push(Attribute::Float),
                Attribute::Vec3 => list.push(Attribute::Float),
                Attribute::Mat3 => list.push(Attribute::Vec3),
                Attribute::Bool => list.push(Attribute::Bool),
                _ => {}
            }
        }

        list
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BufferInput {
    Float(f32),
    Vec2([f32; 2]),
    Vec2u([u32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat2([f32; 4]),
    Mat3([f32; 9]),
    Mat4([f32; 16]),
    Color([f32; 4]),
    Bool(bool),
}

impl BufferInput {
    pub fn size(&self) -> u32 {
        match self {
            BufferInput::Float(_) => 1,
            BufferInput::Vec2(_) => 2,
            BufferInput::Vec2u(_) => 2,
            BufferInput::Vec3(_) => 3,
            BufferInput::Vec4(_) => 4,
            BufferInput::Mat2(_) => 4,
            BufferInput::Mat3(_) => 9,
            BufferInput::Mat4(_) => 16,
            BufferInput::Color(_) => 4,
            BufferInput::Bool(_) => 1,
        }
    }

    pub fn pad(&self) -> BufferInput {
        match self {
            BufferInput::Float(v) => BufferInput::Vec2([*v; 2]),
            BufferInput::Vec3(v) => BufferInput::Vec4([v[0], v[1], v[2], 0.0]),
            BufferInput::Mat3(v) => {
                let mut mat4 = [0.0; 16];
                for x in 0..3 {
                    for y in 0..3 {
                        mat4[x * 4 + y] = v[x * 3 + y];
                    }
                }

                BufferInput::Mat4(mat4)
            }
            BufferInput::Bool(v) => match v {
                true => BufferInput::Vec2u([1, 1]),
                false => BufferInput::Vec2u([0, 0]),
            },
            _ => *self,
        }
    }

    pub fn need_padding(&self) -> bool {
        match self {
            BufferInput::Float(_) => true,
            BufferInput::Vec3(_) => true,
            BufferInput::Mat3(_) => true,
            BufferInput::Bool(_) => true,
            _ => false,
        }
    }
}

impl From<Attribute> for BufferInput {
    fn from(attribute: Attribute) -> BufferInput {
        match attribute {
            Attribute::Float => BufferInput::Float(0.0),
            Attribute::Vec2 => BufferInput::Vec2([0.0; 2]),
            Attribute::Vec2u => BufferInput::Vec2u([0; 2]),
            Attribute::Vec3 => BufferInput::Vec3([0.0; 3]),
            Attribute::Vec4 => BufferInput::Vec4([0.0; 4]),
            Attribute::Mat2 => BufferInput::Mat2([0.0; 4]),
            Attribute::Mat3 => BufferInput::Mat3([0.0; 9]),
            Attribute::Mat4 => BufferInput::Mat4([0.0; 16]),
            Attribute::Color => BufferInput::Color([0.0; 4]),
            Attribute::Bool => BufferInput::Bool(false),
            _ => BufferInput::Float(0.0),
        }
    }
}

impl From<&Attribute> for BufferInput {
    fn from(attribute: &Attribute) -> BufferInput {
        From::from(*attribute)
    }
}

pub const PADDING_PREFIX: &str = "__padding__";

#[derive(Clone, Debug)]
pub struct BufferProperty {
    name: String,
    input: BufferInput,
}

impl BufferProperty {
    pub fn new(name: &str, input: BufferInput) -> BufferProperty {
        BufferProperty {
            name: name.to_string(),
            input,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn input(&self) -> &BufferInput {
        &self.input
    }

    pub fn set_input(&mut self, input: BufferInput) {
        self.input = input;
    }

    pub fn add_padding(inputs: &[BufferProperty]) -> Vec<BufferProperty> {
        let mut properties = vec![];

        for property in inputs {
            properties.push(property.clone());

            match property.input() {
                BufferInput::Float(_) => properties.push(BufferProperty::new(
                    &format!("{}{}", PADDING_PREFIX, properties.len()),
                    BufferInput::Float(0.0),
                )),
                BufferInput::Vec3(_) => properties.push(BufferProperty::new(
                    &format!("{}{}", PADDING_PREFIX, properties.len()),
                    BufferInput::Float(0.0),
                )),
                BufferInput::Mat3(_) => properties.push(BufferProperty::new(
                    &format!("{}{}", PADDING_PREFIX, properties.len()),
                    BufferInput::Vec3([0.0; 3]),
                )),
                BufferInput::Bool(_) => properties.push(BufferProperty::new(
                    &format!("{}{}", PADDING_PREFIX, properties.len()),
                    BufferInput::Bool(false),
                )),
                _ => {}
            }
        }

        properties
    }
}

#[derive(Clone, Debug)]
pub struct TextureProperty {
    name: String,
    texture: TextureId,
    dimension: wgpu::TextureViewDimension,
}

impl TextureProperty {
    pub fn new(
        name: &str,
        texture: TextureId,
        dimension: wgpu::TextureViewDimension,
    ) -> TextureProperty {
        TextureProperty {
            name: name.to_string(),
            texture,
            dimension,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn texture(&self) -> TextureId {
        self.texture
    }

    pub fn dimension(&self) -> wgpu::TextureViewDimension {
        self.dimension
    }

    pub fn set_texture(&mut self, texture: TextureId) {
        self.texture = texture;
    }
}

#[derive(Clone, Debug)]
pub struct PropertyBlock {
    inputs: Vec<BufferProperty>,
    textures: Vec<TextureProperty>,
}

impl PropertyBlock {
    pub fn new() -> PropertyBlock {
        PropertyBlock {
            inputs: vec![],
            textures: vec![],
        }
    }

    pub fn inputs(&self) -> &[BufferProperty] {
        &self.inputs
    }

    pub fn textures(&self) -> &[TextureProperty] {
        &self.textures
    }

    pub fn merge(&mut self, other: PropertyBlock) {
        self.inputs.clear();
        self.textures.clear();

        self.inputs.extend(other.inputs.into_iter());
        self.textures.extend(other.textures.into_iter());
    }

    pub fn set_input(&mut self, property: BufferProperty) -> &mut PropertyBlock {
        if let Some(index) = self.inputs.iter().position(|p| p.name() == property.name()) {
            let old = self.inputs[index].input().clone();
            if old.size() != property.input().size() && property.input().need_padding() {}

            self.inputs[index] = property;
        } else {
            self.inputs.push(property);
        }

        self.inputs
            .sort_by(|a, b| a.input().size().cmp(&b.input.size()));

        self
    }

    pub fn set_texture(
        &mut self,
        name: &str,
        id: &TextureId,
        dimension: wgpu::TextureViewDimension,
    ) -> &mut PropertyBlock {
        let property = TextureProperty::new(name, *id, dimension);
        if let Some(index) = self
            .textures
            .iter()
            .position(|p| p.name() == property.name())
        {
            self.textures[index] = property;
        } else {
            self.textures.push(property);
        }

        self
    }
}

pub trait Material: 'static {
    fn blend_mode() -> BlendMode;
    fn model() -> ShaderModel;
    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;
    fn create_bind_group(&self, device: &wgpu::Device, resources: &GpuResources)
        -> wgpu::BindGroup;
}

pub struct UnlitTexture {
    texture: TextureId,
}

impl UnlitTexture {
    pub fn set_texture(&mut self, texture: TextureId) {
        self.texture = texture;
    }
}

impl Material for UnlitTexture {
    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UnlitTexture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        resources: &GpuResources,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UnlitTexture"),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        resources.texture_view(&self.texture).unwrap_or_else(|| {
                            resources
                                .texture_view(&GpuResources::WHITE_TEXTURE.into())
                                .unwrap()
                        }),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        resources.sampler(&self.texture).unwrap_or_else(|| {
                            resources
                                .sampler(&GpuResources::WHITE_TEXTURE.into())
                                .unwrap()
                        }),
                    ),
                },
            ],
        })
    }

    fn blend_mode() -> BlendMode {
        BlendMode::Opaque
    }

    fn model() -> ShaderModel {
        ShaderModel::Unlit
    }
}

pub struct MaterialShader<M: Material> {
    model: ShaderModel,
    blend_mode: BlendMode,
    module: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> MaterialShader<M> {
    pub fn new(device: &wgpu::Device, module: wgpu::ShaderModule) -> MaterialShader<M> {
        MaterialShader {
            module,
            model: M::model(),
            blend_mode: M::blend_mode(),
            layout: M::bind_group_layout(device),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
}
