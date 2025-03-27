use ecs::system::schedule::{Phase, PhaseId, Schedule};
use game::AppTag;

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}

pub struct Process;
impl Phase for Process {
    fn id(&self) -> ecs::system::schedule::PhaseId {
        PhaseId::of::<Self>()
    }

    fn schedule() -> Schedule {
        let mut schedule = Schedule::new(PhaseId::of::<Self>());
        schedule.add_child(ProcessResources::schedule());
        schedule.add_child(ProcessAssets::schedule());
        schedule.add_child(ProcessPipelines::schedule());
        schedule
    }
}

pub struct ProcessResources;
impl Phase for ProcessResources {}
pub struct ProcessAssets;
impl Phase for ProcessAssets {}

pub struct ProcessPipelines;
impl Phase for ProcessPipelines {}

pub struct Queue;
impl Phase for Queue {
    fn id(&self) -> ecs::system::schedule::PhaseId {
        PhaseId::of::<Self>()
    }

    fn schedule() -> Schedule {
        let mut schedule = Schedule::new(PhaseId::of::<Self>());
        schedule.add_child(QueueViews::schedule());
        schedule.add_child(QueueDraws::schedule());
        schedule
    }
}
pub struct QueueViews;
impl Phase for QueueViews {}
pub struct QueueDraws;
impl Phase for QueueDraws {}

pub struct PreRender;
impl Phase for PreRender {}

pub struct Render;
impl Phase for Render {}

pub struct PostRender;
impl Phase for PostRender {}
