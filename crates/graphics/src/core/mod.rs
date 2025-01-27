use game::AppTag;

pub mod color;
pub mod device;
pub mod export;
pub mod viewport;

pub use color::*;
pub use device::*;
pub use export::*;
pub use viewport::*;

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}
