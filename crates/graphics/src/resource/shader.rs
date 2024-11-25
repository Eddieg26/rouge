use crate::{
    asset::{AssetUsage, ExtractError, RenderAsset, RenderAssetExtractor, RenderAssets},
    device::RenderDevice,
    resource::Id,
};
use asset::{
    importer::{DefaultProcessor, ImportContext, Importer},
    io::{AssetIoError, AssetReader},
    Asset, AssetId, AsyncReadExt,
};
use ecs::system::{unlifetime::ReadRes, ArgItem, StaticArg};
use std::{borrow::Cow, sync::Arc};

pub use meta::*;

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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShaderSource {
    Spirv {
        data: Cow<'static, [u32]>,
        meta: ShaderMeta,
    },
    Glsl {
        data: Cow<'static, str>,
        stage: ShaderStage,
    },
    Wgsl {
        data: Cow<'static, str>,
        meta: ShaderMeta,
    },
}

#[derive(Debug)]
pub enum ShaderLoadError {
    Io(AssetIoError),
    InvalidExt(String),
    Parse(String),
}

impl From<wgpu::naga::front::wgsl::ParseError> for ShaderLoadError {
    fn from(err: wgpu::naga::front::wgsl::ParseError) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<wgpu::naga::front::spv::Error> for ShaderLoadError {
    fn from(err: wgpu::naga::front::spv::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<wgpu::naga::front::glsl::Error> for ShaderLoadError {
    fn from(err: wgpu::naga::front::glsl::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<AssetIoError> for ShaderLoadError {
    fn from(err: AssetIoError) -> Self {
        Self::Io(err)
    }
}

impl std::fmt::Display for ShaderLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::InvalidExt(err) => write!(f, "Parse error: {}", err),
            Self::Parse(err) => write!(f, "WGSL parse error: {}", err),
        }
    }
}

impl From<std::io::Error> for ShaderLoadError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(AssetIoError::from(err))
    }
}

impl std::error::Error for ShaderLoadError {}

impl Asset for ShaderSource {}

impl Importer for ShaderSource {
    type Asset = Self;
    type Settings = ();
    type Error = ShaderLoadError;
    type Processor = DefaultProcessor<Self, Self::Settings>;

    fn extensions() -> &'static [&'static str] {
        &["spv", "wgsl", "vert", "frag", "comp"]
    }

    async fn import(
        ctx: &mut ImportContext<'_, Self::Asset, Self::Settings>,
        reader: &mut dyn AssetReader,
    ) -> Result<Self::Asset, Self::Error> {
        use wgpu::naga::{front::*, valid::*};

        let ext = ctx.path().ext();

        match ext {
            Some("spv") => {
                let mut buffer = Vec::new();
                reader
                    .read_to_end(&mut buffer)
                    .await
                    .map_err(ShaderLoadError::from)?;

                let module =
                    spv::parse_u8_slice(&buffer, &wgpu::naga::front::spv::Options::default())
                        .map_err(ShaderLoadError::from)?;
                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                validator
                    .validate(&module)
                    .map_err(|e| ShaderLoadError::Parse(e.to_string()))?;

                let meta = ShaderMeta::from(&module);
                let data = Cow::Owned(buffer.iter().map(|b| *b as u32).collect());

                Ok(ShaderSource::Spirv { data, meta })
            }
            Some("wgsl") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;

                let module = wgsl::parse_str(&data).map_err(ShaderLoadError::from)?;
                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                validator
                    .validate(&module)
                    .map_err(|e| ShaderLoadError::Parse(e.to_string()))?;

                let meta = ShaderMeta::from(&module);
                let data = Cow::Owned(data);

                Ok(ShaderSource::Wgsl { data, meta })
            }
            Some("vert") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Glsl {
                    data: Cow::Owned(data),
                    stage: ShaderStage::Vertex,
                })
            }
            Some("frag") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Glsl {
                    data: Cow::Owned(data),
                    stage: ShaderStage::Fragment,
                })
            }
            Some("comp") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Glsl {
                    data: Cow::Owned(data),
                    stage: ShaderStage::Compute,
                })
            }
            _ => Err(ShaderLoadError::InvalidExt(format!(
                "Invalid extension: {:?}",
                ext
            ))),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Shader {
    #[serde(skip)]
    module: Arc<wgpu::ShaderModule>,

    #[serde(skip)]
    meta: Option<ShaderMeta>,
}

impl<'de> serde::Deserialize<'de> for Shader {
    fn deserialize<D>(_: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom("Deserialization not supported"))
    }
}

impl Shader {
    pub fn create(device: &RenderDevice, source: &ShaderSource) -> Self {
        match source {
            ShaderSource::Spirv { data, meta } => {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::SpirV(data.clone()),
                });

                Self {
                    module: Arc::new(module),
                    meta: Some(meta.clone()),
                }
            }
            ShaderSource::Glsl {
                data: shader,
                stage,
            } => {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: shader.clone(),
                        stage: (*stage).into(),
                        defines: Default::default(),
                    },
                });

                Self {
                    module: Arc::new(module),
                    meta: None,
                }
            }
            ShaderSource::Wgsl { data, meta } => {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(data.clone()),
                });

                Self {
                    module: Arc::new(module),
                    meta: Some(meta.clone()),
                }
            }
        }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    pub fn meta(&self) -> Option<&ShaderMeta> {
        self.meta.as_ref()
    }
}

impl RenderAsset for Shader {
    type Id = Id<Shader>;
}

impl std::ops::Deref for Shader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl RenderAssetExtractor for Shader {
    type Source = ShaderSource;
    type Asset = Shader;
    type Arg = StaticArg<'static, ReadRes<RenderDevice>>;

    fn extract(
        _: &AssetId,
        source: &mut Self::Source,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, ExtractError> {
        Ok(Shader::create(&arg, source))
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Asset>, _: &mut ArgItem<Self::Arg>) {
        assets.remove(&Id::<Shader>::from(id));
    }

    fn usage(_: &AssetId, _: &Self::Source) -> AssetUsage {
        AssetUsage::Discard
    }
}

pub mod meta {
    use std::{borrow::Cow, num::NonZeroU32};

    #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub enum ShaderValue {
        Float,
        UInt,
        SInt,
        Bool,
        Vec2,
        Vec3,
        Vec4,
        Color,
        Mat2,
        Mat3,
        Mat4,
        Array(Box<ShaderValue>, Option<NonZeroU32>),
        Struct(Vec<ShaderValue>),
        Other,
    }

    impl ShaderValue {
        /// Size in bytes
        pub fn size(&self) -> usize {
            match self {
                Self::Float => 4,
                Self::UInt => 4,
                Self::SInt => 4,
                Self::Bool => 1,
                Self::Vec2 => 8,
                Self::Vec3 => 12,
                Self::Vec4 => 16,
                Self::Color => 16,
                Self::Mat2 => 16,
                Self::Mat3 => 36,
                Self::Mat4 => 64,
                Self::Array(value, count) => match count {
                    Some(count) => value.size() * count.get() as usize,
                    None => 0,
                },
                Self::Struct(values) => values.iter().map(|v| v.size()).sum(),
                Self::Other => 0,
            }
        }

        pub fn count(&self) -> Option<NonZeroU32> {
            match self {
                Self::Array(_, count) => *count,
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct BufferLayout {
        values: Vec<ShaderValue>,
    }

    impl BufferLayout {
        pub fn new() -> Self {
            Self { values: Vec::new() }
        }

        pub fn add(&mut self, value: ShaderValue) {
            self.values.push(value);
        }

        pub fn iter(&self) -> impl Iterator<Item = &ShaderValue> {
            self.values.iter()
        }

        pub fn len(&self) -> usize {
            self.values.len()
        }

        pub fn size(&self) -> usize {
            self.values.iter().map(|v| v.size()).sum()
        }

        pub fn aligned_size(&self, alignment: usize) -> usize {
            let size = self.size();
            let remainder = size % alignment;
            if remainder == 0 {
                size
            } else {
                size + alignment - remainder
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum ShaderBindingKind {
        Texture1D,
        Texture2D,
        Texture2DArray,
        Texture3D,
        Texture3DArray,
        TextureCube,
        Sampler {
            compare: bool,
        },
        Uniform {
            layout: BufferLayout,
            count: Option<NonZeroU32>,
        },
        Storage {
            layout: BufferLayout,
            access: StorageAccess,
            count: Option<NonZeroU32>,
        },
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ShaderBinding {
        pub binding: u32,
        pub kind: ShaderBindingKind,
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ShaderBindGroup {
        group: u32,
        bindings: Vec<ShaderBinding>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum BuiltinValue {
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
        WorkGroupId,
        NumWorkGroups,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum ShaderAttribute {
        Align(u32),
        Binding(u32),
        BlendSrc(bool),
        Builtin(BuiltinValue),
        Group(u32),
        Id(u32),
        Location(u32),
        Size(u32),
        WorkGroupSize {
            x: u32,
            y: Option<u32>,
            z: Option<u32>,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum StorageAccess {
        Read,
        Write,
        ReadWrite,
    }

    impl From<wgpu::naga::StorageAccess> for StorageAccess {
        fn from(access: wgpu::naga::StorageAccess) -> Self {
            let load = access.contains(wgpu::naga::StorageAccess::LOAD);
            let store = access.contains(wgpu::naga::StorageAccess::STORE);

            match (load, store) {
                (true, true) => Self::ReadWrite,
                (true, false) => Self::Read,
                (false, true) => Self::Write,
                _ => Self::Read,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ShaderInput {
        pub value: ShaderValue,
        pub attribute: ShaderAttribute,
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ShaderOuput(ShaderInput);
    impl std::ops::Deref for ShaderOuput {
        type Target = ShaderInput;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl std::ops::DerefMut for ShaderOuput {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ShaderMeta {
        entry: Cow<'static, str>,
        inputs: Vec<ShaderInput>,
        outputs: Vec<ShaderOuput>,
        bindings: Vec<ShaderBindGroup>,
        instances: Option<NonZeroU32>,
    }

    impl ShaderMeta {
        pub fn new(entry: impl Into<Cow<'static, str>>) -> Self {
            Self {
                entry: entry.into(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                bindings: Vec::new(),
                instances: None,
            }
        }

        pub fn with_instances(entry: impl Into<Cow<'static, str>>, instances: NonZeroU32) -> Self {
            Self {
                entry: entry.into(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                bindings: Vec::new(),
                instances: Some(instances),
            }
        }

        pub fn entry(&self) -> &str {
            &self.entry
        }

        pub fn inputs(&self) -> &[ShaderInput] {
            &self.inputs
        }

        pub fn outputs(&self) -> &[ShaderOuput] {
            &self.outputs
        }

        pub fn bindings(&self) -> &[ShaderBindGroup] {
            &self.bindings
        }

        pub fn instances(&self) -> Option<NonZeroU32> {
            self.instances
        }

        pub fn add_input(&mut self, value: ShaderValue, attribute: ShaderAttribute) -> &mut Self {
            self.inputs.push(ShaderInput { value, attribute });
            self
        }

        pub fn add_output(&mut self, value: ShaderValue, attribute: ShaderAttribute) -> &mut Self {
            self.outputs
                .push(ShaderOuput(ShaderInput { value, attribute }));
            self
        }

        pub fn add_binding(
            &mut self,
            group: u32,
            binding: u32,
            kind: ShaderBindingKind,
        ) -> &mut Self {
            match self.bindings.iter_mut().find(|b| b.group == group) {
                Some(bind_group) => bind_group.bindings.push(ShaderBinding { binding, kind }),
                None => self.bindings.push(ShaderBindGroup {
                    group,
                    bindings: vec![ShaderBinding { binding, kind }],
                }),
            }

            self
        }

        pub fn set_instances(&mut self, instances: NonZeroU32) -> &mut Self {
            self.instances = Some(instances);
            self
        }
    }

    impl From<wgpu::naga::ScalarKind> for BufferLayout {
        fn from(value: wgpu::naga::ScalarKind) -> Self {
            let mut layout = BufferLayout::new();
            match value {
                wgpu::naga::ScalarKind::Sint => layout.add(ShaderValue::SInt),
                wgpu::naga::ScalarKind::Uint => layout.add(ShaderValue::UInt),
                wgpu::naga::ScalarKind::Float => layout.add(ShaderValue::Float),
                wgpu::naga::ScalarKind::Bool => layout.add(ShaderValue::Bool),
                wgpu::naga::ScalarKind::AbstractInt => layout.add(ShaderValue::Other),
                wgpu::naga::ScalarKind::AbstractFloat => layout.add(ShaderValue::Other),
            }

            layout
        }
    }

    impl From<wgpu::naga::ScalarKind> for ShaderValue {
        fn from(value: wgpu::naga::ScalarKind) -> Self {
            match value {
                wgpu::naga::ScalarKind::Sint => Self::SInt,
                wgpu::naga::ScalarKind::Uint => Self::UInt,
                wgpu::naga::ScalarKind::Float => Self::Float,
                wgpu::naga::ScalarKind::Bool => Self::Bool,
                wgpu::naga::ScalarKind::AbstractInt => Self::Other,
                wgpu::naga::ScalarKind::AbstractFloat => Self::Other,
            }
        }
    }

    impl From<&wgpu::naga::VectorSize> for BufferLayout {
        fn from(value: &wgpu::naga::VectorSize) -> Self {
            let mut layout = BufferLayout::new();
            layout.add(ShaderValue::from(value));
            layout
        }
    }

    impl From<&wgpu::naga::VectorSize> for ShaderValue {
        fn from(value: &wgpu::naga::VectorSize) -> Self {
            match value {
                wgpu::naga::VectorSize::Bi => Self::Vec2,
                wgpu::naga::VectorSize::Tri => Self::Vec3,
                wgpu::naga::VectorSize::Quad => Self::Vec4,
            }
        }
    }

    impl From<(&wgpu::naga::VectorSize, &wgpu::naga::VectorSize)> for ShaderValue {
        fn from(value: (&wgpu::naga::VectorSize, &wgpu::naga::VectorSize)) -> Self {
            match (value.0, value.1) {
                (wgpu::naga::VectorSize::Bi, wgpu::naga::VectorSize::Bi) => Self::Mat2,
                (wgpu::naga::VectorSize::Tri, wgpu::naga::VectorSize::Tri) => Self::Mat3,
                (wgpu::naga::VectorSize::Quad, wgpu::naga::VectorSize::Quad) => Self::Mat4,
                _ => Self::Other,
            }
        }
    }

    impl ShaderValue {
        fn from_array(
            module: &wgpu::naga::Module,
            ty: &wgpu::naga::Handle<wgpu::naga::Type>,
            size: &wgpu::naga::ArraySize,
        ) -> Option<ShaderValue> {
            let ty_inner = &module.types[*ty].inner;

            let count = match size {
                wgpu::naga::ArraySize::Dynamic => None,
                wgpu::naga::ArraySize::Constant(count) => Some(*count),
            };

            let kind = match ty_inner {
                wgpu::naga::TypeInner::Scalar(scalar) => Some(ShaderValue::from(scalar.kind)),
                wgpu::naga::TypeInner::Vector { size, .. } => Some(ShaderValue::from(size)),
                wgpu::naga::TypeInner::Matrix { columns, rows, .. } => {
                    Some(ShaderValue::from((columns, rows)))
                }
                wgpu::naga::TypeInner::Array { base, size, .. } => {
                    ShaderValue::from_array(module, base, size)
                }
                wgpu::naga::TypeInner::Struct { members, .. } => {
                    ShaderValue::from_struct(module, members)
                }
                _ => None,
            }?;

            Some(ShaderValue::Array(Box::new(kind), count))
        }

        fn from_struct(
            module: &wgpu::naga::Module,
            members: &[wgpu::naga::StructMember],
        ) -> Option<ShaderValue> {
            let mut value = Vec::new();

            for member in members {
                match &module.types[member.ty].inner {
                    wgpu::naga::TypeInner::Scalar(scalar) => {
                        value.push(ShaderValue::from(scalar.kind))
                    }
                    wgpu::naga::TypeInner::Vector { size, .. } => {
                        value.push(ShaderValue::from(size))
                    }
                    wgpu::naga::TypeInner::Matrix { columns, rows, .. } => {
                        value.push(ShaderValue::from((columns, rows)))
                    }
                    wgpu::naga::TypeInner::Array { base, size, .. } => {
                        value.push(ShaderValue::from_array(module, base, size)?)
                    }
                    wgpu::naga::TypeInner::Struct { members, .. } => {
                        value.push(ShaderValue::from_struct(module, members)?)
                    }
                    _ => continue,
                }
            }

            Some(ShaderValue::Struct(value))
        }
    }

    impl From<&wgpu::naga::Module> for ShaderMeta {
        fn from(module: &wgpu::naga::Module) -> Self {
            let entry = module.entry_points[0].name.clone();

            let mut meta = ShaderMeta::new(entry);

            for (_, value) in module.global_variables.iter() {
                if let Some(binding) = &value.binding {
                    let kind = match &module.types[value.ty].inner {
                        wgpu::naga::TypeInner::Scalar(scalar) => match value.space {
                            wgpu::naga::AddressSpace::Uniform => ShaderBindingKind::Uniform {
                                layout: scalar.kind.into(),
                                count: None,
                            },
                            wgpu::naga::AddressSpace::Storage { access } => {
                                let access = access.into();
                                ShaderBindingKind::Storage {
                                    layout: scalar.kind.into(),
                                    access,
                                    count: None,
                                }
                            }
                            _ => continue,
                        },
                        wgpu::naga::TypeInner::Vector { size, .. } => match value.space {
                            wgpu::naga::AddressSpace::Uniform => ShaderBindingKind::Uniform {
                                layout: size.into(),
                                count: None,
                            },
                            wgpu::naga::AddressSpace::Storage { access } => {
                                let access = access.into();
                                ShaderBindingKind::Storage {
                                    layout: size.into(),
                                    access,
                                    count: None,
                                }
                            }
                            _ => continue,
                        },
                        wgpu::naga::TypeInner::Matrix { columns, rows, .. } => match value.space {
                            wgpu::naga::AddressSpace::Uniform => {
                                let mut layout = BufferLayout::new();
                                layout.add((columns, rows).into());
                                ShaderBindingKind::Uniform {
                                    layout,
                                    count: None,
                                }
                            }
                            wgpu::naga::AddressSpace::Storage { access } => {
                                let mut layout = BufferLayout::new();
                                layout.add((columns, rows).into());
                                let access = access.into();
                                ShaderBindingKind::Storage {
                                    layout,
                                    access,
                                    count: None,
                                }
                            }
                            _ => continue,
                        },
                        wgpu::naga::TypeInner::Array { base, size, .. } => match value.space {
                            wgpu::naga::AddressSpace::Uniform => {
                                let mut layout = BufferLayout::new();
                                let mut count = None;
                                if let Some(value) = ShaderValue::from_array(module, base, size) {
                                    count = value.count();
                                    layout.add(value);
                                }

                                ShaderBindingKind::Uniform { layout, count }
                            }
                            wgpu::naga::AddressSpace::Storage { access } => {
                                let mut layout = BufferLayout::new();
                                let mut count = None;
                                if let Some(value) = ShaderValue::from_array(module, base, size) {
                                    count = value.count();
                                    layout.add(value);
                                }
                                let access = access.into();
                                ShaderBindingKind::Storage {
                                    layout,
                                    access,
                                    count,
                                }
                            }
                            _ => continue,
                        },
                        wgpu::naga::TypeInner::Struct { members, .. } => match value.space {
                            wgpu::naga::AddressSpace::Uniform => {
                                let mut layout = BufferLayout::new();
                                let mut count = None;
                                if let Some(value) = ShaderValue::from_struct(module, &members) {
                                    count = value.count();
                                    layout.add(value);
                                }
                                ShaderBindingKind::Uniform { layout, count }
                            }
                            wgpu::naga::AddressSpace::Storage { access } => {
                                let mut layout = BufferLayout::new();
                                let mut count = None;
                                if let Some(value) = ShaderValue::from_struct(module, &members) {
                                    count = value.count();
                                    layout.add(value);
                                }
                                let access = access.into();
                                ShaderBindingKind::Storage {
                                    layout,
                                    access,
                                    count,
                                }
                            }
                            _ => continue,
                        },
                        wgpu::naga::TypeInner::Image { dim, arrayed, .. } => match dim {
                            wgpu::naga::ImageDimension::D1 => ShaderBindingKind::Texture1D,
                            wgpu::naga::ImageDimension::D2 => match arrayed {
                                true => ShaderBindingKind::Texture2DArray,
                                false => ShaderBindingKind::Texture2D,
                            },
                            wgpu::naga::ImageDimension::D3 => ShaderBindingKind::Texture3D,
                            wgpu::naga::ImageDimension::Cube => match arrayed {
                                true => ShaderBindingKind::TextureCube,
                                false => ShaderBindingKind::Texture2D,
                            },
                        },
                        wgpu::naga::TypeInner::Sampler { comparison } => {
                            ShaderBindingKind::Sampler {
                                compare: *comparison,
                            }
                        }
                        _ => continue,
                    };

                    meta.add_binding(binding.group, binding.binding, kind);
                }
            }

            meta
        }
    }
}
