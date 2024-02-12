use rouge_ecs::bits::AsBytes;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

pub mod actions;
pub mod config;
pub mod database;
pub mod error;
pub mod loader;
pub mod plugin;
pub mod storage;

pub trait Asset: Send + Sync + 'static {}

impl Asset for () {}

pub trait Settings:
    serde::Serialize + serde::de::DeserializeOwned + AsBytes + Default + Send + Sync + 'static
{
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DefaultSettings {
    value: u8,
}

impl Settings for DefaultSettings {}

impl AsBytes for DefaultSettings {
    fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = u8::from_bytes(bytes)?;
        Some(DefaultSettings { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new(id: u64) -> Self {
        AssetId(id)
    }

    pub fn gen() -> Self {
        let id = ulid::Ulid::new();
        let mut state = DefaultHasher::new();
        id.hash(&mut state);

        AssetId(state.finish())
    }
}

impl AsBytes for AssetId {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = u64::from_bytes(bytes)?;
        Some(AssetId(id))
    }
}

impl serde::Serialize for AssetId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'a> serde::Deserialize<'a> for AssetId {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        let id = id.parse().map_err(serde::de::Error::custom)?;
        Ok(AssetId(id))
    }
}

impl std::ops::Deref for AssetId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetType(u64);

impl AssetType {
    pub fn new<A: Asset>() -> Self {
        let mut state = DefaultHasher::new();
        std::any::TypeId::of::<A>().hash(&mut state);

        AssetType(state.finish())
    }
}

impl AsBytes for AssetType {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let ty = u64::from_bytes(bytes)?;
        Some(AssetType(ty))
    }
}

impl std::ops::Deref for AssetType {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetDependency {
    id: AssetId,
    ty: AssetType,
}

impl AssetDependency {
    pub fn new<A: Asset>(id: AssetId) -> Self {
        AssetDependency {
            id,
            ty: AssetType::new::<A>(),
        }
    }

    pub fn raw(id: AssetId, ty: AssetType) -> Self {
        AssetDependency { id, ty }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn ty(&self) -> &AssetType {
        &self.ty
    }
}

impl AsBytes for AssetDependency {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes.extend_from_slice(&self.ty.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let ty = AssetType::from_bytes(&bytes[8..16])?;
        Some(AssetDependency { id, ty })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetRef<A: Asset> {
    id: AssetId,
    _ty: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetRef<A> {
    pub fn new(id: AssetId) -> Self {
        AssetRef {
            id,
            _ty: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<A: Asset> std::ops::Deref for AssetRef<A> {
    type Target = AssetId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<A: Asset> std::ops::DerefMut for AssetRef<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}

impl<A: Asset> Into<AssetDependency> for AssetRef<A> {
    fn into(self) -> AssetDependency {
        AssetDependency::new::<A>(self.id)
    }
}

impl<A: Asset> Into<AssetDependency> for &AssetRef<A> {
    fn into(self) -> AssetDependency {
        AssetDependency::new::<A>(self.id)
    }
}

impl<A: Asset> AsBytes for AssetRef<A> {
    fn to_bytes(&self) -> Vec<u8> {
        let dependency = AssetDependency::new::<A>(self.id);
        dependency.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let dependency = AssetDependency::from_bytes(bytes)?;
        Some(AssetRef {
            id: dependency.id,
            _ty: std::marker::PhantomData,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetPath {
    Id(AssetId),
    Path(PathBuf),
}

impl Into<AssetPath> for AssetId {
    fn into(self) -> AssetPath {
        AssetPath::Id(self)
    }
}

impl Into<AssetPath> for &AssetId {
    fn into(self) -> AssetPath {
        AssetPath::Id(*self)
    }
}

impl Into<AssetPath> for String {
    fn into(self) -> AssetPath {
        AssetPath::Path(self.into())
    }
}

impl Into<AssetPath> for &str {
    fn into(self) -> AssetPath {
        AssetPath::Path(self.into())
    }
}

impl Into<AssetPath> for PathBuf {
    fn into(self) -> AssetPath {
        AssetPath::Path(self)
    }
}

impl Into<AssetPath> for &PathBuf {
    fn into(self) -> AssetPath {
        AssetPath::Path(self.clone())
    }
}

impl Into<AssetPath> for &Path {
    fn into(self) -> AssetPath {
        AssetPath::Path(self.to_path_buf())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetInfo {
    id: AssetId,
    ty: AssetType,
    checksum: u64,
}

impl AssetInfo {
    pub fn new<A: Asset>(id: AssetId, settings: &[u8], data: &[u8]) -> Self {
        AssetInfo {
            id,
            ty: AssetType::new::<A>(),
            checksum: Self::calculate_checksum::<A>(settings, data),
        }
    }

    pub fn with_checksum<A: Asset>(id: AssetId, checksum: u64) -> Self {
        AssetInfo {
            id,
            ty: AssetType::new::<A>(),
            checksum,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }

    pub fn calculate_checksum<A: Asset>(settings: &[u8], data: &[u8]) -> u64 {
        let mut state = DefaultHasher::new();
        let ty = std::any::TypeId::of::<A>();
        ty.hash(&mut state);
        settings.hash(&mut state);
        data.hash(&mut state);

        state.finish()
    }
}

impl AsBytes for AssetInfo {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes.extend_from_slice(&self.ty.to_bytes());
        bytes.extend_from_slice(&self.checksum.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let ty = AssetType::from_bytes(&bytes[8..16])?;
        let checksum = u64::from_bytes(&bytes[16..24])?;
        Some(AssetInfo { id, ty, checksum })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct AssetMetadata<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetMetadata<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        AssetMetadata { id, settings }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }
}

impl<'a, S: Settings> serde::Deserialize<'a> for AssetMetadata<S> {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Settings,
        }

        struct AssetMetadataVisitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'a, S: Settings> serde::de::Visitor<'a> for AssetMetadataVisitor<S> {
            type Value = AssetMetadata<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetMetadata")
            }

            fn visit_seq<V: serde::de::SeqAccess<'a>>(
                self,
                mut seq: V,
            ) -> Result<Self::Value, V::Error> {
                let id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let settings = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                Ok(AssetMetadata { id, settings })
            }

            fn visit_map<V: serde::de::MapAccess<'a>>(
                self,
                mut map: V,
            ) -> Result<Self::Value, V::Error> {
                let mut id = None;
                let mut settings = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::Settings => {
                            if settings.is_some() {
                                return Err(serde::de::Error::duplicate_field("settings"));
                            }
                            settings = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let settings =
                    settings.ok_or_else(|| serde::de::Error::missing_field("settings"))?;

                Ok(AssetMetadata { id, settings })
            }
        }

        deserializer.deserialize_struct(
            "AssetMetadata",
            &["id", "settings"],
            AssetMetadataVisitor(std::marker::PhantomData),
        )
    }
}

impl<S: Settings> AsBytes for AssetMetadata<S> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_bytes());
        bytes.extend_from_slice(&self.settings.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let id = AssetId::from_bytes(&bytes[0..8])?;
        let settings = S::from_bytes(&bytes[8..])?;
        Some(AssetMetadata { id, settings })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HashId(u64);

impl HashId {
    pub fn new<H: Hash>(value: &H) -> Self {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);

        HashId(state.finish())
    }
}

impl std::ops::Deref for HashId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> Either<A, B> {
    pub fn left(&self) -> Option<&A> {
        match self {
            Either::Left(a) => Some(a),
            _ => None,
        }
    }

    pub fn right(&self) -> Option<&B> {
        match self {
            Either::Right(b) => Some(b),
            _ => None,
        }
    }
}
