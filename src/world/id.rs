use std::sync::atomic::AtomicU32;

pub static mut WORLD_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldId(u32);

impl WorldId {
    pub fn new() -> Self {
        let id = unsafe { WORLD_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) };
        Self(id)
    }
}
impl std::ops::Deref for WorldId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}