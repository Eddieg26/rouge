use crate::core::{
    component::Component,
    registry::{Record, Registry, Type},
};
use std::{
    alloc::Layout,
    any::{type_name, TypeId},
};

pub struct ComponentMeta {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
}

impl ComponentMeta {
    pub fn new<C: Component>() -> Self {
        let name = type_name::<C>();
        let layout = Layout::new::<C>();
        let type_id = TypeId::of::<C>();

        Self {
            name,
            layout,
            type_id,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }
}

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

impl Record for ComponentMeta {
    type Type = ComponentId;
}

pub type Components = Registry<ComponentMeta>;
