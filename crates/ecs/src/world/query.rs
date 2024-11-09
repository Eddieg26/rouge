use super::{cell::WorldCell, World};
use crate::{
    archetype::{table::SelectedRow, Archetype, Archetypes},
    core::{
        component::{Component, ComponentId},
        entity::Entity,
        resource::ResourceId,
        Type,
    },
    system::{AccessType, SystemArg, WorldAccess},
};
use indexmap::IndexSet;

pub struct QueryCursor<'a> {
    archetypes: IndexSet<&'a Archetype>,
    entity: usize,
    archetype: usize,
    row: Option<SelectedRow<'a>>,
}

impl<'a> QueryCursor<'a> {
    pub fn new(archetypes: IndexSet<&'a Archetype>) -> Self {
        Self {
            archetypes,
            entity: 0,
            archetype: 0,
            row: None,
        }
    }

    pub fn archetypes(&self) -> &IndexSet<&'a Archetype> {
        &self.archetypes
    }

    pub fn row(&self) -> Option<&SelectedRow<'a>> {
        self.row.as_ref()
    }

    pub fn entity(&self) -> Option<&Entity> {
        self.archetypes[self.archetype]
            .table()
            .entities()
            .get_index(self.entity)
    }

    pub fn next(&mut self) {
        if self.archetype >= self.archetypes.len() {
            return;
        }

        let archetype = &self.archetypes[self.archetype];
        if self.entity >= archetype.table().entities().len() {
            self.archetype += 1;
            self.entity = 0;
            self.row = None;
            self.next();
        } else {
            let entity = archetype.table().entities()[self.entity];
            self.row = archetype.table().select(&entity);
            self.entity += 1;
        }
    }
}

pub trait BaseQuery: Send + Sync {
    type Item<'a>: Send + Sync;

    fn init(_: &World, _: &mut QueryState) {}
    fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a>;
    fn access() -> Vec<WorldAccess>;
}

impl<C: Component> BaseQuery for &C {
    type Item<'a> = &'a C;

    fn init(world: &World, state: &mut QueryState) {
        let id = ComponentId::of::<C>();
        #[cfg(debug_assertions)]
        {
            let index = world.registry().index_of(&*id);
            if !world.access().read(index) {
                let meta = world.registry().get(&*id);
                panic!("Component {} is already borrowed", meta.name());
            }
        }
        state.add_component(id);
    }

    fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
        world.get().get_component(entity).unwrap()
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::Component {
            ty: ComponentId::of::<C>(),
            access: AccessType::Read,
        }]
    }
}

impl<C: Component> BaseQuery for &mut C {
    type Item<'a> = &'a mut C;

    fn init(world: &World, state: &mut QueryState) {
        let id = ComponentId::of::<C>();
        #[cfg(debug_assertions)]
        {
            let index = world.registry().index_of(&*id);
            if !world.access().write(index) {
                let meta = world.registry().get(&*id);
                panic!("Component {} is already borrowed", meta.name());
            }
        }
        state.add_component(id);
    }

    fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
        world.get_mut().get_component_mut(entity).unwrap()
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::Component {
            ty: ComponentId::of::<C>(),
            access: AccessType::Write,
        }]
    }
}

impl<C: Component> BaseQuery for Option<&C> {
    type Item<'a> = Option<&'a C>;

    fn init(world: &World, state: &mut QueryState) {
        <&C as BaseQuery>::init(world, state);
    }

    fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
        world.get().get_component(entity)
    }

    fn access() -> Vec<WorldAccess> {
        <&C as BaseQuery>::access()
    }
}

impl<C: Component> BaseQuery for Option<&mut C> {
    type Item<'a> = Option<&'a mut C>;

    fn init(world: &World, state: &mut QueryState) {
        <&mut C as BaseQuery>::init(world, state);
    }

    fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
        world.get_mut().get_component_mut(entity)
    }

    fn access() -> Vec<WorldAccess> {
        <&mut C as BaseQuery>::access()
    }
}

impl BaseQuery for Entity {
    type Item<'a> = Entity;

    fn fetch<'a>(_: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
        entity
    }

    fn access() -> Vec<WorldAccess> {
        vec![]
    }
}

pub trait QueryFilter {
    fn init(world: &World, state: &mut QueryState);
}

pub struct With<C: Component> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: Component> QueryFilter for With<C> {
    fn init(_: &World, state: &mut QueryState) {
        state.add_component(ComponentId::of::<C>());
    }
}

pub struct Not<C: Component> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: Component> QueryFilter for Not<C> {
    fn init(_: &World, state: &mut QueryState) {
        state.exclude(ComponentId::of::<C>());
    }
}

impl QueryFilter for () {
    fn init(_: &World, _: &mut QueryState) {}
}

pub struct Query<'a, Q: BaseQuery, F: QueryFilter = ()> {
    world: WorldCell<'a>,
    cursor: QueryCursor<'a>,
    _marker: std::marker::PhantomData<(Q, F)>,
}

impl<'a, Q: BaseQuery, F: QueryFilter> Query<'a, Q, F> {
    pub fn new(world: WorldCell<'a>) -> Self {
        let mut state = QueryState::new();
        Q::init(world.get(), &mut state);
        F::init(world.get(), &mut state);

        let archetypes = world
            .get()
            .archetypes()
            .query(state.components(), state.excluded());
        let cursor = QueryCursor::new(archetypes);

        Self {
            world,
            cursor,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn filter(&self, entities: &'a [Entity]) -> FilterQuery<'a, Q, F> {
        FilterQuery::new(self.world, entities)
    }
}

pub struct FilterQuery<'a, Q: BaseQuery, F: QueryFilter = ()> {
    world: WorldCell<'a>,
    archetypes: IndexSet<&'a Archetype>,
    archetype: usize,
    entity: usize,
    entities: &'a [Entity],
    _marker: std::marker::PhantomData<(Q, F)>,
}

impl<'a, Q: BaseQuery, F: QueryFilter> FilterQuery<'a, Q, F> {
    pub fn new(world: WorldCell<'a>, entities: &'a [Entity]) -> Self {
        let mut state = QueryState::new();
        Q::init(world.get(), &mut state);
        F::init(world.get(), &mut state);

        let archetypes = world
            .get()
            .archetypes()
            .query(state.components(), state.excluded());

        Self {
            world,
            archetypes,
            archetype: 0,
            entity: 0,
            entities,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct QueryState {
    components: Vec<ComponentId>,
    excluded: Vec<ComponentId>,
}

impl QueryState {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            excluded: Vec::new(),
        }
    }

    pub fn add_component(&mut self, component: ComponentId) {
        self.components.push(component);
    }

    pub fn exclude(&mut self, component: ComponentId) {
        self.excluded.push(component);
    }

    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }

    pub fn excluded(&self) -> &[ComponentId] {
        &self.excluded
    }
}

impl<'a, Q: BaseQuery, F: QueryFilter> Iterator for Query<'a, Q, F> {
    type Item = Q::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.archetypes().is_empty() {
            return None;
        }

        let entity = self.cursor.entity()?;
        let item = Q::fetch(self.world, *entity);
        self.cursor.next();
        Some(item)
    }
}

impl<'a, Q: BaseQuery, F: QueryFilter> Iterator for FilterQuery<'a, Q, F> {
    type Item = Q::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.archetype >= self.archetypes.len() {
            return None;
        } else if self.entity >= self.entities.len() {
            self.archetype += 1;
            self.entity = 0;
            return self.next();
        }

        let entity = self.entities[self.entity];
        let item = Q::fetch(self.world, entity);
        self.entity += 1;
        Some(item)
    }
}

impl<Q: BaseQuery, F: QueryFilter> SystemArg for Query<'_, Q, F> {
    type Item<'a> = Query<'a, Q, F>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        Query::new(world)
    }

    fn access() -> Vec<WorldAccess> {
        let mut access = Q::access();
        access.push(WorldAccess::Resource {
            ty: ResourceId::dynamic(Type::of::<Archetypes>()),
            access: AccessType::Read,
            send: true,
        });

        access
    }
}

#[macro_export]
macro_rules! impl_base_query_for_tuples {
    ($(($($name:ident),+)),+) => {
        $(
            impl<$($name: BaseQuery),+> BaseQuery for ($($name,)+) {
                type Item<'a> = ($($name::Item<'a>,)+);

                fn init(world: &World, state: &mut QueryState) {
                    $(
                        $name::init(world, state);
                    )+
                }

                fn fetch<'a>(world: WorldCell<'a>, entity: Entity) -> Self::Item<'a> {
                    ($($name::fetch(world, entity),)+)
                }

                fn access() -> Vec<WorldAccess> {
                    let mut metas = Vec::new();
                    $(
                        metas.extend($name::access());
                    )+
                    metas
                }
            }
        )+
    };
}

#[macro_export]
macro_rules! impl_filter_query_for_tuple {
    ($($filter:ident),*) => {
        impl<$($filter: FilterQuery),*> FilterQuery for ($($filter,)*) {
            fn init(world: &World, state: &mut QueryState) {
                $(
                    $filter::init(world, state);
                )*
            }
        }
    };
}

impl_base_query_for_tuples!((A, B));
impl_base_query_for_tuples!((A, B, C));
impl_base_query_for_tuples!((A, B, C, D));
impl_base_query_for_tuples!((A, B, C, D, E));
impl_base_query_for_tuples!((A, B, C, D, E, F));
impl_base_query_for_tuples!((A, B, C, D, E, F, G));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q));
