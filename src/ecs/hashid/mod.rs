use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub struct HashId;

impl HashId {
    pub fn new() -> u64 {
        let uuid = uuid::Uuid::new_v4();
        let mut hasher = DefaultHasher::new();
        uuid.hash(&mut hasher);

        hasher.finish()
    }

    pub fn id<T: Hash>(value: T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);

        hasher.finish()
    }
}
