use asset::{
    importer::{DefaultProcessor, ImportContext, Importer},
    io::{AssetIoError, AssetReader},
    Asset, AssetId, AsyncReadExt,
};
use ecs::system::{unlifetime::ReadRes, ArgItem, StaticArg};
use std::{borrow::Cow, sync::Arc};

use crate::core::{
    device::RenderDevice,
    asset::{AssetUsage, ExtractError, RenderAsset, RenderAssetExtractor, RenderAssets},
};

use super::Id;

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
    Spirv(Cow<'static, [u32]>),
    Glsl {
        shader: Cow<'static, str>,
        stage: ShaderStage,
    },
    Wgsl(Cow<'static, str>),
}

#[derive(Debug)]
pub enum ShaderLoadError {
    Io(AssetIoError),
    Parse(String),
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
            Self::Parse(err) => write!(f, "Parse error: {}", err),
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
        let ext = ctx.path().ext();

        match ext {
            Some("spv") => {
                let mut buffer = Vec::new();
                reader
                    .read_to_end(&mut buffer)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Spirv(Cow::Owned(
                    buffer.iter().map(|b| *b as u32).collect(),
                )))
            }
            Some("wgsl") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Wgsl(Cow::Owned(data)))
            }
            Some("vert") => {
                let mut data = String::new();
                reader
                    .read_to_string(&mut data)
                    .await
                    .map_err(ShaderLoadError::from)?;
                Ok(ShaderSource::Glsl {
                    shader: Cow::Owned(data),
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
                    shader: Cow::Owned(data),
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
                    shader: Cow::Owned(data),
                    stage: ShaderStage::Compute,
                })
            }
            _ => Err(ShaderLoadError::Parse(format!(
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
        let module = match source {
            ShaderSource::Spirv(data) => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::SpirV(data.clone()),
                })
            }
            ShaderSource::Glsl { shader, stage } => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Glsl {
                        shader: shader.clone(),
                        stage: (*stage).into(),
                        defines: Default::default(),
                    },
                })
            }
            ShaderSource::Wgsl(data) => device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(data.clone()),
            }),
        };

        Self {
            module: Arc::new(module),
        }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
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
    use std::borrow::Cow;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ShaderBinding {
        Texture2D,
        Texture2DArray,
        Texture3D,
        Texture3DArray,
        TextureCube,
        Sampler,
        Uniform {
            layout: BufferLayout,
        },
        Storage {
            layout: BufferLayout,
            read_write: bool,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ShaderInput {
        pub value: ShaderValue,
        pub attribute: ShaderAttribute,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ShaderMeta {
        entry: Cow<'static, str>,
        inputs: Vec<ShaderInput>,
        outputs: Vec<ShaderOuput>,
        bindings: Vec<ShaderBinding>,
    }

    impl ShaderMeta {
        pub fn new(entry: &'static str) -> Self {
            Self {
                entry: Cow::Borrowed(entry),
                inputs: Vec::new(),
                outputs: Vec::new(),
                bindings: Vec::new(),
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

        pub fn bindings(&self) -> &[ShaderBinding] {
            &self.bindings
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

        pub fn add_binding(&mut self, binding: ShaderBinding) -> &mut Self {
            self.bindings.push(binding);
            self
        }
    }
}
