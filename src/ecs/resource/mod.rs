use std::any::Any;

pub mod id;
pub mod manager;

pub use id::*;

pub trait Resource: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
