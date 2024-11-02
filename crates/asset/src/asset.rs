use ecs::core::{internal::blob::BlobCell, resource::Resource, Type};
use hashbrown::HashMap;
use serde::ser::SerializeStruct;
use std::{hash::Hash, marker::PhantomData};
use uuid::Uuid;

pub trait Asset: Send + Sync + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

pub trait Settings: Default + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

impl Asset for () {}
impl Settings for () {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetId(Uuid);

impl AssetId {
    pub const ZERO: AssetId = AssetId(Uuid::nil());

    pub fn new<A: Asset>() -> Self {
        let ty = AssetType::of::<A>();
        let mut id = Uuid::new_v4();

        unsafe {
            let addr = std::ptr::addr_of_mut!(id) as *mut u32;
            std::ptr::write(addr, ty.value().to_be());
        }

        Self(id)
    }

    pub fn ty(&self) -> AssetType {
        let bytes: [u8; 4] = self.0.as_bytes()[0..4].try_into().unwrap();
        let ty = u32::from_be_bytes(bytes);
        AssetType::dynamic(ty)
    }

    pub fn from<A: Asset>(id: Uuid) -> Self {
        let ty = AssetType::of::<A>();
        let mut id = id;

        unsafe {
            let addr = std::ptr::addr_of_mut!(id) as *mut u32;
            std::ptr::write(addr, ty.value().to_be());
        }

        Self(id)
    }

    pub fn with_type(&self, ty: AssetType) -> Self {
        let mut id = self.0;

        unsafe {
            let addr = std::ptr::addr_of_mut!(id) as *mut u32;
            std::ptr::write(addr, ty.value().to_be());
        }

        Self(id)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind {
    Main,
    Sub,
}

impl ToString for AssetId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AssetType(u32);

impl AssetType {
    pub fn of<A: Asset>() -> Self {
        Self(Type::of::<A>().value())
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn dynamic(ty: u32) -> Self {
        Self(ty)
    }
}

pub struct AssetRef<A: Asset> {
    id: AssetId,
    _marker: PhantomData<A>,
}

impl<A: Asset> AssetRef<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: PhantomData::default(),
        }
    }

    pub fn from(id: Uuid) -> Self {
        Self {
            id: AssetId::from::<A>(id),
            _marker: PhantomData::default(),
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }
}

impl<A: Asset> Default for AssetRef<A> {
    fn default() -> Self {
        Self {
            id: AssetId::new::<A>(),
            _marker: PhantomData::default(),
        }
    }
}

impl<A: Asset> std::ops::Deref for AssetRef<A> {
    type Target = AssetId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<A: Asset> Copy for AssetRef<A> {}
impl<A: Asset> Clone for AssetRef<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: PhantomData::default(),
        }
    }
}

impl<A: Asset> PartialEq for AssetRef<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A: Asset> Eq for AssetRef<A> {}

impl<A: Asset> Hash for AssetRef<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<A: Asset> serde::Serialize for AssetRef<A> {
    fn serialize<Ser>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        self.id.serialize(ser)
    }
}

impl<'de, A: Asset> serde::Deserialize<'de> for AssetRef<A> {
    fn deserialize<D>(de: D) -> Result<AssetRef<A>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(AssetRef {
            id: AssetId::deserialize(de)?,
            _marker: PhantomData::default(),
        })
    }
}

impl<A: Asset> Into<AssetId> for AssetRef<A> {
    fn into(self) -> AssetId {
        self.id
    }
}

impl<A: Asset> Into<Uuid> for AssetRef<A> {
    fn into(self) -> Uuid {
        self.id.value()
    }
}

pub struct AssetMetadata<A: Asset, S: Settings> {
    id: AssetId,
    settings: S,
    _marker: PhantomData<A>,
}

impl<A: Asset, S: Settings> AssetMetadata<A, S> {
    pub fn new(id: Uuid, settings: S) -> Self {
        Self {
            id: AssetId::from::<A>(id),
            settings,
            _marker: PhantomData::default(),
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn into(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<A: Asset, S: Settings> Default for AssetMetadata<A, S> {
    fn default() -> Self {
        Self {
            id: AssetId::new::<A>(),
            settings: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<A: Asset, S: Settings> std::ops::Deref for AssetMetadata<A, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.settings
    }
}

impl<A: Asset, S: Settings> std::ops::DerefMut for AssetMetadata<A, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.settings
    }
}

impl<A: Asset, S: Settings> serde::Serialize for AssetMetadata<A, S> {
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

impl<'de, A: Asset, S: Settings> serde::Deserialize<'de> for AssetMetadata<A, S> {
    fn deserialize<D>(de: D) -> Result<AssetMetadata<A, S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Settings,
        }

        struct Visitor<A: Asset, S: Settings>(std::marker::PhantomData<(A, S)>);

        impl<'de, A: Asset, S: Settings> serde::de::Visitor<'de> for Visitor<A, S> {
            type Value = AssetMetadata<A, S>;

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

                Ok(AssetMetadata {
                    id,
                    settings,
                    _marker: PhantomData::default(),
                })
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
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.assets.insert(id, asset)
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
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

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }

    pub fn clear(&mut self) {
        self.assets.clear()
    }
}

impl<A: Asset> Resource for Assets<A> {}

pub struct ErasedAsset {
    asset: BlobCell,
    ty: AssetType,
}

impl ErasedAsset {
    pub fn new<A: Asset>(asset: A) -> Self {
        Self {
            asset: BlobCell::new(asset),
            ty: AssetType::of::<A>(),
        }
    }

    pub fn asset<A: Asset>(&self) -> Option<&A> {
        if self.ty == AssetType::of::<A>() {
            Some(self.asset.value::<A>())
        } else {
            None
        }
    }

    pub fn asset_mut<A: Asset>(&mut self) -> Option<&mut A> {
        if self.ty == AssetType::of::<A>() {
            Some(self.asset.value_mut::<A>())
        } else {
            None
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn take<A: Asset>(self) -> A {
        assert!(self.ty == AssetType::of::<A>());
        self.asset.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaMode {
    Text,
    Binary,
}
