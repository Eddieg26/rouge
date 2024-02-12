pub use glam as math;
pub mod primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Release,
}
