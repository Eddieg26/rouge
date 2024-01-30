use pack::AsBytes;
use rouge_ecs::{
    macros::Resource,
    system::{ArgItem, SystemArg},
    world::resource::Resource,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

pub mod database;
pub mod filesystem;
pub mod pack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new() -> Self {
        let id = ulid::Ulid::new();
        let mut state = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut state);

        Self(state.finish())
    }

    pub const fn zero() -> Self {
        Self(0)
    }
}

impl From<u64> for AssetId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetType(u64);

impl AssetType {
    pub fn new<A: Asset>() -> Self {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<A>().hash(&mut state);

        Self(state.finish())
    }
}

impl From<u64> for AssetType {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<AssetType> for u64 {
    fn from(value: AssetType) -> Self {
        value.0
    }
}

impl From<std::any::TypeId> for AssetType {
    fn from(value: std::any::TypeId) -> Self {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut state);

        Self(state.finish())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetMetadata<S: LoadSettings> {
    id: AssetId,
    settings: S,
}

impl<S: LoadSettings> AssetMetadata<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self { id, settings }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }
}

impl<'a, S: LoadSettings> serde::Deserialize<'a> for AssetMetadata<S> {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Settings,
        }

        struct AssetMetadataVisitor<S: LoadSettings>(std::marker::PhantomData<S>);

        impl<'a, S: LoadSettings> serde::de::Visitor<'a> for AssetMetadataVisitor<S> {
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

        const FIELDS: &[&str] = &["id", "settings"];
        deserializer.deserialize_struct(
            "AssetMetadata
        ",
            FIELDS,
            AssetMetadataVisitor(std::marker::PhantomData),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevMode {
    Development,
    Release,
}

pub trait Asset: Send + Sync + 'static {}

impl Asset for () {}

pub trait LoadSettings:
    Send + Sync + Sized + serde::Serialize + serde::de::DeserializeOwned + Default + 'static
{
}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: LoadSettings;
    type Arg: SystemArg;
    type Processor: AssetProcessor<Asset = Self::Asset, Arg = Self::Arg>;

    fn load(ctx: LoadContext<Self::Settings>, data: &[u8]) -> Self::Asset;
    fn unload<'a>(
        id: AssetId,
        asset: Self::Asset,
        settings: &'a Self::Settings,
        arg: ArgItem<'a, Self::Arg>,
    );
    fn extensions() -> &'static [&'static str];
}

pub trait AssetProcessor: Send + Sync + 'static {
    type Asset: Asset;
    type Arg: SystemArg;

    fn process<'a>(id: AssetId, asset: &mut Self::Asset, arg: ArgItem<'a, Self::Arg>);
}

impl AssetProcessor for () {
    type Asset = ();
    type Arg = ();

    fn process<'a>(_: AssetId, _: &mut Self::Asset, _: ArgItem<'a, Self::Arg>) {}
}

#[derive(Resource)]
pub struct Assets<A: Asset> {
    assets: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, asset: A) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: AssetId) -> Option<&A> {
        self.assets.get(&id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut A> {
        self.assets.get_mut(&id)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<A> {
        self.assets.remove(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.assets.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }
}

#[derive(Resource)]
pub struct AssetMetadatas<S: LoadSettings> {
    metadata: HashMap<AssetId, AssetMetadata<S>>,
}

impl<S: LoadSettings> AssetMetadatas<S> {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, metadata: AssetMetadata<S>) {
        self.metadata.insert(id, metadata);
    }

    pub fn get(&self, id: AssetId) -> Option<&AssetMetadata<S>> {
        self.metadata.get(&id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut AssetMetadata<S>> {
        self.metadata.get_mut(&id)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<AssetMetadata<S>> {
        self.metadata.remove(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.metadata.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetMetadata<S>)> {
        self.metadata.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut AssetMetadata<S>)> {
        self.metadata.iter_mut()
    }
}

pub struct LoadContext<'a, S: LoadSettings> {
    dev_mode: DevMode,
    path: PathBuf,
    id: AssetId,
    settings: &'a S,
}

impl<'a, S: LoadSettings> LoadContext<'a, S> {
    pub fn new(dev_mode: DevMode, path: PathBuf, id: AssetId, settings: &'a S) -> Self {
        Self {
            dev_mode,
            path,
            id,
            settings,
        }
    }

    pub fn dev_mode(&self) -> DevMode {
        self.dev_mode
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        self.settings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Resource)]
pub struct AssetSerializer<A: Asset> {
    serialize: fn(&A) -> Vec<u8>,
    deserialize: fn(&[u8]) -> A,
}

impl<A: Asset> AssetSerializer<A> {
    pub fn new(serialize: fn(&A) -> Vec<u8>, deserialize: fn(&[u8]) -> A) -> Self {
        Self {
            serialize,
            deserialize,
        }
    }

    pub fn serialize(&self, asset: &A) -> Vec<u8> {
        (self.serialize)(asset)
    }

    pub fn deserialize(&self, bytes: &[u8]) -> A {
        (self.deserialize)(bytes)
    }
}
