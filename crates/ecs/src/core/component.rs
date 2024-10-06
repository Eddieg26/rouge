use super::Type;

pub trait Component: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(Type);
impl ComponentId {
    pub fn of<C: Component>() -> Self {
        Self(Type::of::<C>())
    }
}
impl std::ops::Deref for ComponentId {
    type Target = Type;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<Type> for ComponentId {
    fn into(self) -> Type {
        self.0
    }
}

impl Into<Type> for &ComponentId {
    fn into(self) -> Type {
        self.0
    }
}
