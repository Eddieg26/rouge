use super::entity::EntityId;
use std::any::Any;

pub trait Registry {
    fn contains(&self, id: &EntityId) -> bool;
    fn remove(&mut self, id: &EntityId);
    fn clear(&mut self);

    fn update(&mut self);
    fn enable(&mut self, id: &EntityId);
    fn disable(&mut self, id: &EntityId);
    fn destroy(&mut self, id: &EntityId);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
