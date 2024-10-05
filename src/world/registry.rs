use super::{builtin::events::ComponentUpdate, World};
use crate::{
    archetype::table::ColumnCell,
    core::{component::Component, entity::Entity, resource::Resource, Type},
    event::{Event, Events},
};
use indexmap::IndexMap;
use std::{alloc::Layout, any::TypeId, sync::Arc};

pub trait MetadataHooks: downcast_rs::Downcast + 'static {}
downcast_rs::impl_downcast!(MetadataHooks);

impl MetadataHooks for () {}

#[derive(Clone, Copy)]
pub struct ComponentHooks {
    on_added: fn(&mut World, Entity),
    on_removed: fn(&mut World, Entity),
    on_replaced: fn(&mut World, Entity, ColumnCell),
}

impl ComponentHooks {
    pub fn new<C: Component>() -> Self {
        Self {
            on_added: |world, entity| {
                world
                    .resource_mut::<Events<ComponentUpdate<C>>>()
                    .add(ComponentUpdate::Added { entity });
            },
            on_removed: |world, entity| {
                world
                    .resource_mut::<Events<ComponentUpdate<C>>>()
                    .add(ComponentUpdate::Removed { entity });
            },
            on_replaced: |world, entity, component| {
                world
                    .resource_mut::<Events<ComponentUpdate<C>>>()
                    .add(ComponentUpdate::Replaced {
                        entity,
                        component: component.into(),
                    });
            },
        }
    }

    #[inline]
    pub fn on_added(&self, world: &mut World, entity: Entity) {
        (self.on_added)(world, entity)
    }

    #[inline]
    pub fn on_removed(&self, world: &mut World, entity: Entity) {
        (self.on_removed)(world, entity)
    }

    #[inline]
    pub fn on_replaced(&self, world: &mut World, entity: Entity, component: ColumnCell) {
        (self.on_replaced)(world, entity, component)
    }
}

impl MetadataHooks for ComponentHooks {}

#[derive(Clone, Copy)]
pub struct EventHooks {
    clear: fn(&mut World),
}

impl EventHooks {
    pub fn new<E: Event>() -> Self {
        Self {
            clear: |world| world.resource_mut::<Events<E>>().clear(),
        }
    }

    pub fn clear(&self, world: &mut World) {
        (self.clear)(world)
    }
}

impl MetadataHooks for EventHooks {}

pub struct Metadata {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
    hooks: Arc<dyn MetadataHooks>,
}

impl Metadata {
    #[inline]
    pub fn new<T: 'static>(hooks: impl MetadataHooks) -> Self {
        Self {
            name: std::any::type_name::<T>(),
            layout: Layout::new::<T>(),
            type_id: TypeId::of::<T>(),
            hooks: Arc::new(hooks),
        }
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    #[inline]
    pub fn hooks(&self) -> &Arc<dyn MetadataHooks> {
        &self.hooks
    }

    #[inline]
    pub fn hooks_as<T: MetadataHooks>(&self) -> &T {
        self.hooks.downcast_ref().expect(&format!(
            "Hooks type mismatch: expected {}, got {}",
            std::any::type_name::<T>(),
            self.name
        ))
    }
}

pub struct Registry {
    metadatas: IndexMap<Type, Metadata>,
}

impl Registry {
    #[inline]
    pub fn new() -> Self {
        Self {
            metadatas: IndexMap::new(),
        }
    }

    #[inline]
    pub fn get(&self, ty: &Type) -> &Metadata {
        self.metadatas
            .get(ty)
            .expect(&format!("Type not registered: {:?}", ty))
    }

    pub fn get_hooks<T: MetadataHooks>(&self, ty: &Type) -> &T {
        self.get(ty).hooks_as()
    }

    #[inline]
    pub fn register_component<C: Component>(&mut self) -> Type {
        self.register::<C>(ComponentHooks::new::<C>())
    }

    #[inline]
    pub fn register_resource<R: Resource>(&mut self) -> Type {
        self.register::<R>(())
    }

    #[inline]
    pub fn register_event<E: Event>(&mut self) -> Type {
        self.register::<E>(EventHooks::new::<E>())
    }

    #[inline]
    pub fn index_of(&self, ty: &Type) -> usize {
        self.metadatas
            .get_index_of(ty)
            .expect(&format!("Type not registered: {:?}", ty))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.metadatas.len()
    }

    fn register<T: 'static>(&mut self, hooks: impl MetadataHooks) -> Type {
        let ty = Type::of::<T>();
        let metadata = Metadata::new::<T>(hooks);
        self.metadatas.insert(ty, metadata);
        ty
    }
}
