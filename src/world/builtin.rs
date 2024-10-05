pub mod actions {
    use super::{
        components::Children,
        events::{Despawned, Spawned},
    };
    use crate::{
        archetype::table::Row,
        core::{component::Component, entity::Entity},
        event::Events,
        world::{action::WorldAction, cell::WorldCell, registry::ComponentExtension},
    };

    pub struct Spawn {
        components: Row,
    }

    impl Spawn {
        pub fn new() -> Self {
            Self {
                components: Row::new(),
            }
        }

        pub fn with<C: Component>(mut self, component: C) -> Self {
            self.components.add_component(component);
            self
        }
    }

    impl WorldAction for Spawn {
        fn execute(self, world: &mut crate::world::World) -> Option<()> {
            let entity = world.spawn();
            let mv = world.add_components(entity, self.components)?;

            world.resource_mut::<Events<Spawned>>().add(entity.into());
            let world = WorldCell::from(world);
            let registry = world.get().registry();
            for id in mv.added {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_added(world.get_mut(), entity);
            }

            for (id, component) in mv.replaced {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_replaced(world.get_mut(), entity, component);
            }

            for (id, component) in mv.removed {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_removed(world.get_mut(), entity, component);
            }

            Some(())
        }
    }

    pub struct Despawn {
        entity: Entity,
    }

    impl From<Entity> for Despawn {
        fn from(entity: Entity) -> Self {
            Self { entity }
        }
    }

    impl WorldAction for Despawn {
        fn execute(self, world: &mut crate::world::World) -> Option<()> {
            let row = world.despawn(self.entity)?;
            world
                .resource_mut::<Events<Despawned>>()
                .add(self.entity.into());
            let world = WorldCell::from(world);
            let registry = world.get().registry();

            if let Some(children) = row.get::<Children>() {
                for child in children {
                    Despawn::from(*child).execute(world.get_mut());
                }
            }

            for (id, component) in row {
                let hooks = registry.get_extension::<ComponentExtension>(&*id);
                hooks.on_removed(world.get_mut(), self.entity, component);
            }

            Some(())
        }
    }

    pub struct AddChild {
        parent: Entity,
        child: Entity,
    }

    impl AddChild {
        pub fn new(parent: Entity, child: Entity) -> Self {
            Self { parent, child }
        }
    }

    impl WorldAction for AddChild {
        fn execute(self, world: &mut crate::world::World) -> Option<()> {
            Some(())
        }
    }

    pub struct RemoveChild {
        parent: Entity,
        child: Entity,
    }

    impl RemoveChild {
        pub fn new(parent: Entity, child: Entity) -> Self {
            Self { parent, child }
        }
    }

    impl WorldAction for RemoveChild {
        fn execute(self, world: &mut crate::world::World) -> Option<()> {
            Some(())
        }
    }
}

pub mod events {
    use crate::{
        core::{component::Component, entity::Entity},
        event::Event,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Spawned(Entity);

    impl Spawned {
        pub fn entity(&self) -> Entity {
            self.0
        }
    }

    impl From<Entity> for Spawned {
        fn from(entity: Entity) -> Self {
            Self(entity)
        }
    }

    impl Event for Spawned {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Despawned(Entity);

    impl Despawned {
        pub fn entity(&self) -> Entity {
            self.0
        }
    }

    impl From<Entity> for Despawned {
        fn from(entity: Entity) -> Self {
            Self(entity)
        }
    }

    impl Event for Despawned {}

    pub enum ComponentUpdate<C: Component> {
        Added { entity: Entity },
        Removed { entity: Entity, component: C },
        Replaced { entity: Entity, component: C },
    }

    impl<C: Component> ComponentUpdate<C> {
        pub fn entity(&self) -> Entity {
            match self {
                Self::Added { entity } => *entity,
                Self::Removed { entity, .. } => *entity,
                Self::Replaced { entity, .. } => *entity,
            }
        }
    }

    impl<C: Component> Event for ComponentUpdate<C> {}
}

pub mod components {
    use crate::core::{component::Component, entity::Entity};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Parent(Entity);
    impl std::ops::Deref for Parent {
        type Target = Entity;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Component for Parent {}

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Children(Vec<Entity>);

    impl Children {
        pub fn new() -> Self {
            Self(Vec::new())
        }

        pub fn add(&mut self, entity: Entity) {
            self.0.push(entity);
        }

        pub fn remove(&mut self, entity: Entity) {
            self.0.retain(|&e| e != entity);
        }

        pub fn contains(&self, entity: Entity) -> bool {
            self.0.contains(&entity)
        }

        pub fn index(&self, entity: Entity) -> Option<usize> {
            self.0.iter().position(|&e| e == entity)
        }

        pub fn iter(&self) -> impl Iterator<Item = &Entity> + '_ {
            self.0.iter()
        }
    }

    impl IntoIterator for Children {
        type Item = Entity;
        type IntoIter = std::vec::IntoIter<Entity>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    impl<'a> IntoIterator for &'a Children {
        type Item = &'a Entity;
        type IntoIter = std::slice::Iter<'a, Entity>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.iter()
        }
    }

    impl Component for Children {}
}
