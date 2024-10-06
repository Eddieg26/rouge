use super::{builtin::events::Spawned, cell::WorldCell, registry::ComponentExtension, World};
use crate::{
    archetype::{table::Row, Archetypes},
    core::{component::Component, entity::Entity, resource::ResourceId, Type},
    system::{AccessType, SystemArg, WorldAccess},
};
use indexmap::IndexMap;

pub struct Spawner<'a> {
    world: &'a mut World,
    entities: IndexMap<Entity, Row>,
}

impl<'a> Spawner<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            entities: IndexMap::new(),
        }
    }

    pub fn spawn<'b>(&'b mut self) -> SpawnedEntity<'b, 'a> {
        SpawnedEntity::new(self)
    }

    pub fn done(self) {}
}

impl<'a> Drop for Spawner<'a> {
    fn drop(&mut self) {
        let world = WorldCell::from(&self.world);
        let registry = world.get().registry();
        let mut spawned = vec![];
        for (entity, components) in self.entities.drain(..) {
            let mov = match world.get_mut().add_components(entity, components) {
                Some(mov) => mov,
                None => continue,
            };

            for id in mov.added {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_added(world.get_mut(), entity);
            }

            for (id, component) in mov.replaced {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_replaced(world.get_mut(), entity, component);
            }

            for (id, component) in mov.removed {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_removed(world.get_mut(), entity, component);
            }

            spawned.push(entity);
        }

        world.get_mut().invoke_event(Spawned::from(spawned));
    }
}

impl SystemArg for Spawner<'_> {
    type Item<'a> = Spawner<'a>;

    fn get<'a>(world: &'a WorldCell) -> Self::Item<'a> {
        Spawner::new(world.get_mut())
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::Resource {
            ty: ResourceId::dynamic(Type::of::<Archetypes>()),
            access: AccessType::Write,
            send: true,
        }]
    }
}

pub struct SpawnedEntity<'a, 'b: 'a> {
    entity: Entity,
    components: Option<Row>,
    spawner: &'a mut Spawner<'b>,
}

impl<'a, 'b: 'a> SpawnedEntity<'a, 'b> {
    pub fn new(spawner: &'a mut Spawner<'b>) -> Self {
        let entity = spawner.world.entities_mut().spawn();
        Self {
            entity,
            components: Some(Row::new()),
            spawner,
        }
    }

    pub fn with<C: Component>(mut self, component: C) -> Self {
        self.components.as_mut().unwrap().add_component(component);
        self
    }

    pub fn add_component<C: Component>(&mut self, component: C) -> &mut Self {
        self.components.as_mut().unwrap().add_component(component);
        self
    }

    pub fn done(self) -> Entity {
        self.entity
    }
}

impl<'a, 'b: 'a> Drop for SpawnedEntity<'a, 'b> {
    fn drop(&mut self) {
        self.spawner
            .entities
            .insert(self.entity, self.components.take().unwrap());
    }
}
