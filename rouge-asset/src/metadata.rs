use rouge_ecs::{macros::Resource, world::resource::Resource};
use std::collections::HashMap;

use crate::AssetId;

pub trait LoadSettings:
    Send + Sync + Sized + serde::Serialize + serde::de::DeserializeOwned + Default + 'static
{
}

impl LoadSettings for () {}

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
