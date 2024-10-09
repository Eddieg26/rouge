use crate::io::SourceId;
use ecs::core::{internal::blob::BlobCell, resource::Resource, Type};
use hashbrown::{hash_map, HashMap};
use serde::ser::SerializeStruct;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub trait Asset: serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

pub trait Settings: Default + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetId(Uuid);

impl AssetId {
    pub const ZERO: AssetId = AssetId(Uuid::nil());

    pub fn new<A: Asset>() -> Self {
        let ty = Type::of::<A>();
        let id = Uuid::new_v4();

        unsafe {
            let bytes = id.as_bytes() as *const [u8; 16] as *mut [u8; 16];
            let bytes = bytes.as_mut().unwrap();
            let bytes = &mut bytes[0..4];

            bytes.copy_from_slice(&ty.value().to_ne_bytes());
        }

        Self(id)
    }

    pub fn ty(&self) -> AssetType {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.0.as_bytes()[0..4]);
        AssetType::dynamic(Type::dynamic(u32::from_ne_bytes(bytes)))
    }

    pub fn raw(id: Uuid) -> Self {
        Self(id)
    }
}

impl ToString for AssetId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetType(Type);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        Self(Type::of::<A>())
    }

    pub fn dynamic(ty: Type) -> Self {
        Self(ty)
    }
}

impl Into<Type> for AssetType {
    fn into(self) -> Type {
        self.0
    }
}

#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetPath {
    Id { source: SourceId, id: AssetId },
    Path { source: SourceId, path: PathBuf },
}

impl From<AssetId> for AssetPath {
    fn from(id: AssetId) -> Self {
        AssetPath::Id {
            source: SourceId::Default,
            id,
        }
    }
}

impl From<&AssetId> for AssetPath {
    fn from(value: &AssetId) -> Self {
        AssetPath::Id {
            source: SourceId::Default,
            id: *value,
        }
    }
}

impl<A: AsRef<Path>> From<A> for AssetPath {
    fn from(value: A) -> Self {
        AssetPath::Path {
            source: SourceId::Default,
            path: value.as_ref().to_path_buf(),
        }
    }
}

pub struct AssetSettings<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetSettings<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self { id, settings }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn into(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<S: Settings> std::ops::Deref for AssetSettings<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.settings
    }
}

impl<S: Settings> std::ops::DerefMut for AssetSettings<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.settings
    }
}

impl<S: Settings> serde::Serialize for AssetSettings<S> {
    fn serialize<Ser>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let mut object = ser.serialize_struct("Metadata", 2)?;
        object.serialize_field("id", &self.id)?;
        object.serialize_field("settings", &self.settings)?;
        object.end()
    }
}

impl<'de, S: Settings> serde::Deserialize<'de> for AssetSettings<S> {
    fn deserialize<D>(de: D) -> Result<AssetSettings<S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Settings,
        }

        struct Visitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'de, S: Settings> serde::de::Visitor<'de> for Visitor<S> {
            type Value = AssetSettings<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetSettings")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
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

                Ok(AssetSettings { id, settings })
            }
        }

        const FIELDS: &[&str] = &["id", "settings"];
        de.deserialize_struct("Metadata", FIELDS, Visitor(std::marker::PhantomData))
    }
}

pub struct Assets<A: Asset> {
    assets: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn add(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn ids(&self) -> hash_map::Keys<'_, AssetId, A> {
        self.assets.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> + '_ {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> + '_ {
        self.assets.iter_mut()
    }

    pub fn drain(&mut self) -> hash_map::Drain<'_, AssetId, A> {
        self.assets.drain()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

impl<A: Asset> Default for Assets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

impl<A: Asset> Resource for Assets<A> {}

impl<'a, A: Asset> IntoIterator for &'a Assets<A> {
    type Item = (&'a AssetId, &'a A);
    type IntoIter = hash_map::Iter<'a, AssetId, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.assets.iter()
    }
}

impl<'a, A: Asset> IntoIterator for &'a mut Assets<A> {
    type Item = (&'a AssetId, &'a mut A);
    type IntoIter = hash_map::IterMut<'a, AssetId, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.assets.iter_mut()
    }
}

pub struct ErasedAsset {
    ty: AssetType,
    asset: BlobCell,
}

impl ErasedAsset {
    pub fn new<A: Asset>(asset: A) -> Self {
        Self {
            ty: AssetType::of::<A>(),
            asset: BlobCell::new(asset),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn value<A: Asset>(&self) -> &A {
        assert_eq!(self.ty, AssetType::of::<A>());
        self.asset.value()
    }

    pub fn value_mut<A: Asset>(&mut self) -> &mut A {
        assert_eq!(self.ty, AssetType::of::<A>());
        self.asset.value_mut()
    }

    pub fn into<A: Asset>(self) -> A {
        assert_eq!(self.ty, AssetType::of::<A>());
        self.asset.into()
    }
}

pub struct ErasedSettings {
    ty: Type,
    settings: BlobCell,
}

impl ErasedSettings {
    pub fn new<S: Settings>(settings: AssetSettings<S>) -> Self {
        Self {
            ty: Type::of::<S>(),
            settings: BlobCell::new(settings),
        }
    }

    pub fn ty(&self) -> Type {
        self.ty
    }

    pub fn value<S: Settings>(&self) -> &AssetSettings<S> {
        assert_eq!(self.ty, Type::of::<S>());
        self.settings.value()
    }

    pub fn value_mut<S: Settings>(&mut self) -> &mut AssetSettings<S> {
        assert_eq!(self.ty, Type::of::<S>());
        self.settings.value_mut()
    }

    pub fn into<S: Settings>(self) -> AssetSettings<S> {
        assert_eq!(self.ty, Type::of::<S>());
        self.settings.into()
    }
}
