use crate::storage::{blob::Blob, ptr::Ptr};
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    hash::{Hash, Hasher},
};

pub trait Resource: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceType(u64);

impl ResourceType {
    pub fn new<T: Resource>() -> Self {
        Self(hash_id(&TypeId::of::<T>()))
    }

    pub fn dynamic(value: u64) -> Self {
        Self(value)
    }

    pub fn is<T: Resource>(&self) -> bool {
        self.0 == hash_id(&TypeId::of::<T>())
    }
}

impl From<&TypeId> for ResourceType {
    fn from(type_id: &TypeId) -> Self {
        Self(hash_id(type_id))
    }
}

impl From<TypeId> for ResourceType {
    fn from(type_id: TypeId) -> Self {
        Self(hash_id(&type_id))
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        TypeId::of::<Self>().fmt(f)
    }
}

fn hash_id(id: &std::any::TypeId) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

pub struct Resources {
    resources: HashMap<ResourceType, ResourceData>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn insert<R: Resource>(&mut self, resource: R) {
        self.resources
            .insert(ResourceType::new::<R>(), ResourceData::new(resource));
    }

    pub fn get<R: Resource>(&self) -> &R {
        let ty = ResourceType::new::<R>();
        let res = self.resources.get(&ty).expect("Resource doesn't exist.");
        res.get::<R>()
    }

    pub fn get_mut<R: Resource>(&self) -> &mut R {
        let ty = ResourceType::new::<R>();
        let res = self.resources.get(&ty).expect("Resource doesn't exist.");

        res.get_mut::<R>()
    }
}

pub struct ResourceData {
    data: Blob,
}

impl ResourceData {
    pub fn new<R: Resource>(resource: R) -> Self {
        let mut data = Blob::new::<R>();
        data.push(resource);

        ResourceData { data }
    }

    pub fn ptr<'a>(&'a self) -> Ptr<'a> {
        self.data.ptr()
    }

    pub fn get<R: Resource>(&self) -> &R {
        self.data.get::<R>(0).unwrap()
    }

    pub fn get_mut<R: Resource>(&self) -> &mut R {
        self.data.get_mut::<R>(0).unwrap()
    }
}
