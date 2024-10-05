pub mod actions {
    use super::events::Spawned;
    use crate::{
        archetype::table::Row, core::component::Component, event::Events,
        world::action::WorldAction,
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
        fn execute(self, world: &mut crate::world::World) {
            let entity = world.spawn();
            if let Some(mv) = world.add_components(entity, self.components) {
                world.resource_mut::<Events<Spawned>>().add(entity.into());
            }
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
        Removed { entity: Entity },
        Replaced { entity: Entity, component: C },
    }

    impl<C: Component> ComponentUpdate<C> {
        pub fn entity(&self) -> Entity {
            match self {
                Self::Added { entity } => *entity,
                Self::Removed { entity } => *entity,
                Self::Replaced { entity, .. } => *entity,
            }
        }
    }

    impl<C: Component> Event for ComponentUpdate<C> {}
}
