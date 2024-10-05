use super::{builtin::events::ComponentUpdate, World};
use crate::{
    archetype::table::ColumnCell,
    core::{component::Component, entity::Entity, resource::Resource, Type},
    event::{Event, Events},
};
use indexmap::IndexMap;
use std::{alloc::Layout, any::TypeId, sync::Arc};

pub trait MetadataExtension: downcast_rs::Downcast + 'static {}
downcast_rs::impl_downcast!(MetadataExtension);

impl MetadataExtension for () {}

#[derive(Clone, Copy)]
pub struct ComponentExtension {
    on_added: fn(&mut World, Entity),
    on_removed: fn(&mut World, Entity, ColumnCell),
    on_replaced: fn(&mut World, Entity, ColumnCell),
}

impl ComponentExtension {
    pub fn new<C: Component>() -> Self {
        Self {
            on_added: |world, entity| {
                world
                    .resource_mut::<Events<ComponentUpdate<C>>>()
                    .add(ComponentUpdate::Added { entity });
            },
            on_removed: |world, entity, component| {
                world
                    .resource_mut::<Events<ComponentUpdate<C>>>()
                    .add(ComponentUpdate::Removed {
                        entity,
                        component: component.into(),
                    });
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
    pub fn on_removed(&self, world: &mut World, entity: Entity, component: ColumnCell) {
        (self.on_removed)(world, entity, component)
    }

    #[inline]
    pub fn on_replaced(&self, world: &mut World, entity: Entity, component: ColumnCell) {
        (self.on_replaced)(world, entity, component)
    }
}

impl MetadataExtension for ComponentExtension {}

#[derive(Clone, Copy)]
pub struct EventExtension {
    clear: fn(&mut World),
}

impl EventExtension {
    pub fn new<E: Event>() -> Self {
        Self {
            clear: |world| world.resource_mut::<Events<E>>().clear(),
        }
    }

    pub fn clear(&self, world: &mut World) {
        (self.clear)(world)
    }
}

impl MetadataExtension for EventExtension {}

pub struct Metadata {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
    extension: Arc<dyn MetadataExtension>,
}

impl Metadata {
    #[inline]
    pub fn new<T: 'static>(extension: impl MetadataExtension) -> Self {
        Self {
            name: std::any::type_name::<T>(),
            layout: Layout::new::<T>(),
            type_id: TypeId::of::<T>(),
            extension: Arc::new(extension),
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
    pub fn extension(&self) -> &Arc<dyn MetadataExtension> {
        &self.extension
    }

    #[inline]
    pub fn extension_as<T: MetadataExtension>(&self) -> &T {
        self.extension.downcast_ref().expect(&format!(
            "Extension type mismatch: expected {}, got {}",
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

    pub fn get_extension<T: MetadataExtension>(&self, ty: &Type) -> &T {
        self.get(ty).extension_as()
    }

    #[inline]
    pub fn register_component<C: Component>(&mut self) -> Type {
        self.register::<C>(ComponentExtension::new::<C>())
    }

    #[inline]
    pub fn register_resource<R: Resource>(&mut self) -> Type {
        self.register::<R>(())
    }

    #[inline]
    pub fn register_event<E: Event>(&mut self) -> Type {
        self.register::<E>(EventExtension::new::<E>())
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

    fn register<T: 'static>(&mut self, hooks: impl MetadataExtension) -> Type {
        let ty = Type::of::<T>();
        let metadata = Metadata::new::<T>(hooks);
        self.metadatas.insert(ty, metadata);
        ty
    }
}
