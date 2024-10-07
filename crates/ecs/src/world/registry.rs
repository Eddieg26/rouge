use super::{builtin::events::ComponentUpdate, World};
use crate::{
    archetype::table::ColumnCell,
    core::{component::Component, entity::Entity, resource::Resource, Type},
    event::Events,
};
use indexmap::IndexMap;
use std::{alloc::Layout, any::TypeId, sync::Arc};

pub trait MetadataExtension: downcast_rs::Downcast + Send + Sync + 'static {}
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

    pub fn on_added(&self, world: &mut World, entity: Entity) {
        (self.on_added)(world, entity)
    }

    pub fn on_removed(&self, world: &mut World, entity: Entity, component: ColumnCell) {
        (self.on_removed)(world, entity, component)
    }

    pub fn on_replaced(&self, world: &mut World, entity: Entity, component: ColumnCell) {
        (self.on_replaced)(world, entity, component)
    }
}

impl MetadataExtension for ComponentExtension {}

pub struct Metadata {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
    extension: Arc<dyn MetadataExtension>,
}

impl Metadata {
    pub fn new<T: 'static>(extension: impl MetadataExtension) -> Self {
        Self {
            name: std::any::type_name::<T>(),
            layout: Layout::new::<T>(),
            type_id: TypeId::of::<T>(),
            extension: Arc::new(extension),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn extension(&self) -> &Arc<dyn MetadataExtension> {
        &self.extension
    }

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
    pub fn new() -> Self {
        Self {
            metadatas: IndexMap::new(),
        }
    }

    pub fn get(&self, ty: &Type) -> &Metadata {
        self.metadatas
            .get(ty)
            .expect(&format!("Type not registered: {:?}", ty))
    }

    pub fn get_extension<T: MetadataExtension>(&self, ty: &Type) -> &T {
        self.get(ty).extension_as()
    }

    pub fn register_component<C: Component>(&mut self) -> Type {
        self.register::<C>(ComponentExtension::new::<C>())
    }

    pub fn register_resource<R: Resource>(&mut self) -> Type {
        self.register::<R>(())
    }

    pub fn index_of(&self, ty: &Type) -> usize {
        self.metadatas
            .get_index_of(ty)
            .expect(&format!("Type not registered: {:?}", ty))
    }

    pub fn len(&self) -> usize {
        self.metadatas.len()
    }

    pub fn contains(&self, ty: &Type) -> bool {
        self.metadatas.contains_key(ty)
    }

    fn register<T: 'static>(&mut self, hooks: impl MetadataExtension) -> Type {
        let ty = Type::of::<T>();
        if !self.contains(&ty) {
            let metadata = Metadata::new::<T>(hooks);
            self.metadatas.insert(ty, metadata);
        }

        ty
    }
}
