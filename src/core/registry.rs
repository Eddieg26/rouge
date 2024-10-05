use indexmap::IndexMap;
use std::{any::TypeId, fmt::Debug, hash::Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Type(u32);
impl Type {
    pub fn of<R: 'static>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        TypeId::of::<R>().hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn dynamic(ty: u32) -> Self {
        Self(ty)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<TypeId> for Type {
    fn from(type_id: TypeId) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub trait Record: 'static {
    type Type: Into<Type> + Debug + Hash + Eq + Copy + 'static;
}

pub struct Registry<R: Record> {
    records: IndexMap<R::Type, R>,
}

impl<R: Record> Registry<R> {
    pub fn new() -> Self {
        Self {
            records: IndexMap::new(),
        }
    }

    pub fn register(&mut self, ty: R::Type, record: R) {
        self.records.insert(ty, record);
    }

    pub fn get(&self, ty: &R::Type) -> Option<&R> {
        self.records.get(ty)
    }

    pub fn get_mut(&mut self, ty: &R::Type) -> Option<&mut R> {
        self.records.get_mut(ty)
    }

    pub fn index(&self, ty: &R::Type) -> usize {
        self.records
            .get_index_of(ty)
            .expect(format!("Record not registered: {:?}", ty).as_str())
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.records.values()
    }
}
