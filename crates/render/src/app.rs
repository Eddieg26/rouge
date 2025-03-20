use ecs::system::schedule::Phase;
use game::AppTag;

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}

pub struct Process;
impl Phase for Process {}
pub struct Queue;
impl Phase for Queue {}
pub struct Render;
impl Phase for Render {}
