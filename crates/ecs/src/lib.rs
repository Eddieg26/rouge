pub mod archetype;
pub mod core;
pub mod event;
pub mod system;
pub mod task;
pub mod world;

pub use core::*;

pub mod prelude {
    pub use crate::archetype::*;
    pub use crate::core::*;
    pub use crate::event::*;
    pub use crate::system::*;
    pub use crate::task::*;
    pub use crate::world::*;
}

pub mod derive {
    pub use ecs_macros::*;
}
