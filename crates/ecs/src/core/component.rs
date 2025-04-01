use std::any::TypeId;

pub trait Component: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(TypeId);
impl ComponentId {
    pub fn of<C: Component>() -> Self {
        Self(TypeId::of::<C>())
    }
}
impl std::ops::Deref for ComponentId {
    type Target = TypeId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TypeId> for ComponentId {
    fn from(type_id: TypeId) -> Self {
        Self(type_id)
    }
}
