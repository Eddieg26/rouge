use super::{scene::SceneManager, schedule::SchedulePlan, Game};
use crate::ecs::World;

pub struct InitGame;

impl InitGame {
    pub fn execute(ctx: &mut GameContext) {
        ctx.global_plan.run(GamePhase::Init, ctx.world);
        ctx.plan.run(GamePhase::Init, ctx.world);
    }
}

pub struct StartScene;

impl StartScene {
    pub fn execute(ctx: &mut GameContext) {
        let mut has_current = false;
        ctx.world
            .resource::<SceneManager>()
            .current()
            .and_then(|scene| {
                std::mem::swap(ctx.plan, &mut scene.plan());
                scene.start(ctx.world);
                has_current = true;
                Some(())
            });

        if has_current {
            ctx.global_plan.run(GamePhase::StartScene, ctx.world);
            ctx.plan.run(GamePhase::StartScene, ctx.world);
        }
    }
}

pub struct Update;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GamePhase {
    Init,
    StartScene,
    PreUpdate,
    Update,
    PostUpdate,
    PreRender,
    Render,
    PostRender,
    EndFrame,
    EndScene,
    Shutdown,
}

pub struct GameContext<'a> {
    pub world: &'a mut World,
    pub global_plan: &'a mut SchedulePlan,
    pub plan: &'a mut SchedulePlan,
}

impl<'a> GameContext<'a> {
    pub fn new(
        world: &'a mut World,
        global_plan: &'a mut SchedulePlan,
        plan: &'a mut SchedulePlan,
    ) -> GameContext<'a> {
        GameContext {
            world,
            global_plan,
            plan,
        }
    }

    pub fn from_game(game: &'a mut Game) -> GameContext<'a> {
        GameContext {
            world: &mut game.world,
            global_plan: &mut game.global_plan,
            plan: &mut game.plan,
        }
    }
}

impl Update {
    pub fn execute(ctx: &mut GameContext) {
        ctx.global_plan.run(GamePhase::PreUpdate, ctx.world);
        ctx.plan.run(GamePhase::PreUpdate, ctx.world);

        ctx.global_plan.run(GamePhase::Update, ctx.world);
        ctx.plan.run(GamePhase::Update, ctx.world);

        ctx.global_plan.run(GamePhase::PostUpdate, ctx.world);
        ctx.plan.run(GamePhase::PostUpdate, ctx.world);
    }
}

pub struct Render;

impl Render {
    pub fn execute(ctx: &mut GameContext) {
        ctx.global_plan.run(GamePhase::PreRender, ctx.world);
        ctx.plan.run(GamePhase::PreRender, ctx.world);

        ctx.global_plan.run(GamePhase::Render, ctx.world);
        ctx.plan.run(GamePhase::Render, ctx.world);

        ctx.global_plan.run(GamePhase::PostRender, ctx.world);
        ctx.plan.run(GamePhase::PostRender, ctx.world);
    }
}

pub struct EndFrame;

impl EndFrame {
    pub fn execute(ctx: &mut GameContext) {
        ctx.global_plan.run(GamePhase::EndFrame, ctx.world);
        ctx.plan.run(GamePhase::EndFrame, ctx.world);
    }
}

pub struct EndScene;

impl EndScene {
    pub fn execute(ctx: &mut GameContext) {
        ctx.plan.run(GamePhase::EndScene, ctx.world);
        ctx.global_plan.run(GamePhase::EndScene, ctx.world);
        if let Some(current) = ctx.world.resource::<SceneManager>().current() {
            current.end(ctx.world);
        }

        ctx.world.resource_mut::<SceneManager>().transition();
    }
}

pub struct Shutdown;

impl Shutdown {
    pub fn execute(ctx: &mut GameContext) {
        ctx.global_plan.run(GamePhase::Shutdown, ctx.world);
        ctx.plan.run(GamePhase::Shutdown, ctx.world);
    }
}
