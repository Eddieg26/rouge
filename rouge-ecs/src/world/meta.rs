use crate::{
    core::{Component, Entity},
    system::observer::{action::ActionOutputs, builtin::RemoveComponent},
};
use std::any::TypeId;

use super::resource::Resource;
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Access {
    Read,
    Write,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccessType {
    None,
    World,
    Component(TypeId),
    Resource(TypeId),
}

impl AccessType {
    pub fn component<C: Component>() -> Self {
        Self::Component(TypeId::of::<C>())
    }

    pub fn resource<R: Resource>() -> Self {
        Self::Resource(TypeId::of::<R>())
    }

    pub fn world() -> Self {
        Self::World
    }

    pub fn none() -> Self {
        Self::None
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AccessMeta {
    ty: AccessType,
    access: Access,
}

impl AccessMeta {
    pub fn new(ty: AccessType, access: Access) -> Self {
        Self { ty, access }
    }

    pub fn from_type(ty: AccessType, access: Access) -> Self {
        Self { ty, access }
    }

    pub fn ty(&self) -> AccessType {
        self.ty
    }

    pub fn access(&self) -> Access {
        self.access
    }

    pub fn pick(reads: &mut Vec<AccessType>, writes: &mut Vec<AccessType>, metas: &[AccessMeta]) {
        for meta in metas {
            match meta.access() {
                Access::Read => reads.push(meta.ty()),
                Access::Write => writes.push(meta.ty()),
            }
        }
    }

    pub fn collect(types: &[AccessType], access: Access) -> Vec<AccessMeta> {
        types
            .iter()
            .map(|&ty| AccessMeta::from_type(ty, access))
            .collect()
    }
}

pub struct ComponentActionMeta {
    on_remove: Box<dyn Fn(&Entity, &mut ActionOutputs)>,
}

impl ComponentActionMeta {
    pub fn new<C: Component>() -> Self {
        Self {
            on_remove: Box::new(|entity, outputs: &mut ActionOutputs| {
                outputs.add::<RemoveComponent<C>>(*entity);
            }),
        }
    }

    pub fn on_remove(&self) -> &dyn Fn(&Entity, &mut ActionOutputs) {
        &self.on_remove
    }
}
