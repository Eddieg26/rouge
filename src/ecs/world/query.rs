use super::World;
use crate::ecs::{
    component::{Component, ComponentType},
    entity::EntityId,
    registry::Registry,
};
use std::{
    any::TypeId,
    cell::{Ref, RefMut},
    marker::PhantomData,
};

pub struct Query<'a, T: BaseQuery> {
    entities: Vec<EntityId>,
    index: usize,
    world: &'a World,
    _marker: &'a PhantomData<T>,
}

impl<T: BaseQuery> Query<'_, T> {
    pub fn new(world: &World) -> Query<T> {
        let entities = T::entities(world);

        Query {
            entities,
            world,
            index: 0,
            _marker: &PhantomData,
        }
    }

    pub fn filter<'a>(world: &'a World, ids: &'a [EntityId]) -> Query<'a, T> {
        let entities = T::entities(world)
            .iter()
            .filter_map(|id| if ids.contains(id) { Some(*id) } else { None })
            .collect::<Vec<_>>();

        Query {
            entities,
            world,
            index: 0,
            _marker: &PhantomData,
        }
    }

    pub fn entity(world: &World, id: EntityId) -> Query<T> {
        Query {
            entities: vec![id],
            world,
            index: 0,
            _marker: &PhantomData,
        }
    }
}

pub trait BaseFetch {}

pub trait Fetch {
    type Type<'a>;

    fn type_id() -> Option<ComponentType>;
    fn negate_type_id() -> Option<ComponentType> {
        None
    }

    fn contains(world: &World, id: &EntityId) -> bool;

    fn get(world: &World, id: EntityId) -> Self::Type<'_>;
}

pub struct Copied<T: Component + Copy> {
    _marker: PhantomData<T>,
}

pub struct Write<T: Component> {
    _marker: PhantomData<T>,
}

impl BaseFetch for EntityId {}

impl Fetch for EntityId {
    type Type<'a> = EntityId;

    fn type_id() -> Option<ComponentType> {
        None
    }

    fn contains(_world: &World, _id: &EntityId) -> bool {
        true
    }

    fn get(_world: &World, id: EntityId) -> Self::Type<'_> {
        id
    }
}

impl<T: Component> BaseFetch for T {}
impl<T: Component> BaseFetch for Write<T> {}
impl<T: Component + Copy> BaseFetch for Copied<T> {}

impl<T: Component> Fetch for T {
    type Type<'a> = Ref<'a, T>;

    fn type_id() -> Option<ComponentType> {
        Some(TypeId::of::<T>().into())
    }

    fn get(world: &World, id: EntityId) -> Self::Type<'_> {
        let registry = world.components::<T>();
        let component = Ref::map(registry, |r| r.get(&id).unwrap());

        component
    }

    fn contains(world: &World, id: &EntityId) -> bool {
        world.components::<T>().contains(id)
    }
}

impl<T: Component> Fetch for Write<T> {
    type Type<'a> = RefMut<'a, T>;

    fn type_id() -> Option<ComponentType> {
        Some(TypeId::of::<T>().into())
    }

    fn get(world: &World, id: EntityId) -> Self::Type<'_> {
        let registry = world.components_mut::<T>();
        let component = RefMut::map(registry, |r| r.get_mut(&id).unwrap());

        component
    }

    fn contains(world: &World, id: &EntityId) -> bool {
        world.components::<T>().contains(id)
    }
}

impl<T: Fetch + BaseFetch> Fetch for Option<T> {
    type Type<'a> = Option<T::Type<'a>>;

    fn type_id() -> Option<ComponentType> {
        None
    }

    fn negate_type_id() -> Option<ComponentType> {
        T::negate_type_id()
    }

    fn contains(world: &World, id: &EntityId) -> bool {
        T::contains(world, id)
    }

    fn get(world: &World, id: EntityId) -> Self::Type<'_> {
        if T::contains(world, &id) {
            Some(T::get(world, id))
        } else {
            None
        }
    }
}

impl<T: Component + Copy> Fetch for Copied<T> {
    type Type<'a> = T;

    fn type_id() -> Option<ComponentType> {
        Some(TypeId::of::<T>().into())
    }

    fn get(world: &World, id: EntityId) -> Self::Type<'_> {
        let registry = world.components::<T>();
        let component = registry.get(&id).unwrap();

        *component
    }

    fn contains(world: &World, id: &EntityId) -> bool {
        world.components::<T>().contains(id)
    }
}

pub trait BaseQuery {
    fn entities(world: &World) -> Vec<EntityId>;
}

macro_rules! impl_base_query {
    ($($type:ident),*) => {
        impl<$($type),*> BaseQuery for ( $($type),* ) where $($type: Fetch), * {
            fn entities(world: &World) -> Vec<EntityId> {
                let _type = vec![$($type::type_id()),*] .iter()
                .filter_map(|i| i.clone())
                .collect::<Vec<_>>();
                world.archetypes().get_component_entities(&_type)
            }
        }
    };
}

macro_rules! impl_query_iterator {
    ($($type:ident),*) => {
        impl<'a, $($type),*> Iterator for Query<'a, ( $($type),* )> where $($type: Fetch), * {
            type Item = ($($type::Type<'a>),*);

            fn next(&mut self) -> Option<Self::Item> {
                let entity_id = self.entities.get(self.index)?;
                self.index += 1;

                Some(($($type::get(self.world, *entity_id)),*))
            }
        }
    };
}

impl_base_query!(A);
impl_base_query!(A, B);
impl_base_query!(A, B, C);

impl_query_iterator!(A);
impl_query_iterator!(A, B);
impl_query_iterator!(A, B, C);
