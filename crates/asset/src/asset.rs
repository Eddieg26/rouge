use ecs::core::Type;
use serde::ser::SerializeStruct;
use std::{hash::Hash, marker::PhantomData};
use uuid::Uuid;

pub trait Asset: Send + Sync + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

pub trait Settings: Default + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static {}

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

    pub fn raw(id: Uuid) -> Self {
        Self(id)
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

pub struct AssetMetadata<A: Asset, S: Settings> {
    id: AssetId,
    settings: S,
    _marker: PhantomData<A>,
}

impl<A: Asset, S: Settings> AssetMetadata<A, S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        Self {
            id,
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
