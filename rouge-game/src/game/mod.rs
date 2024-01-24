use crate::{
    engine::{Engine, Engines, GameEngine},
    plugin::{Plugin, Plugins},
    time::Time,
};
use rouge_ecs::{
    core::Component,
    schedule::{Schedule, SchedulePhase},
    system::{
        observer::{Action, Observers},
        IntoSystem,
    },
    world::{
        resource::{LocalResource, Resource},
        World,
    },
};

pub struct Game {
    world: World,
    plugins: Plugins,
    engines: Engines,
    runner: Option<Box<dyn GameRunner>>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        world.add_resource(Time::new());
        world.add_resource(GameState::new());

        Self {
            world,
            plugins: Plugins::new(),
            engines: Engines::new(),
            runner: None,
        }
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.world.register::<C>();

        self
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.add_resource(resource);

        self
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) -> &mut Self {
        self.world.add_local_resource(resource);

        self
    }

    pub fn add_system<M>(
        &mut self,
        phase: impl SchedulePhase,
        system: impl IntoSystem<M>,
    ) -> &mut Self
    where
        M: Send + Sync + 'static,
    {
        self.world.add_system(phase, system);
        self
    }

    pub fn add_schedule(&mut self, phase: impl SchedulePhase, schedule: Schedule) -> &mut Self {
        self.world.add_schedule(phase, schedule);
        self
    }

    pub fn add_observers<A: Action>(&mut self, observers: Observers<A>) -> &mut Self {
        self.world.add_observers(observers);

        self
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        self.plugins.register(plugin);

        self
    }

    pub fn add_engine<E: Engine>(&mut self, engine: E) -> &mut Self {
        self.engines.register(engine);

        self
    }

    pub fn with_runner<R: GameRunner>(&mut self, runner: R) -> &mut Self {
        self.runner = Some(Box::new(runner));

        self
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.world.resource_mut::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.world.local_resource::<R>()
    }

    pub fn local_resource_mut<R: LocalResource>(&self) -> &mut R {
        self.world.local_resource_mut::<R>()
    }

    pub fn engine<E: Engine>(&self) -> &GameEngine {
        self.engines.get::<E>()
    }

    pub fn engine_mut<E: Engine>(&mut self) -> &mut GameEngine {
        self.engines.get_mut::<E>()
    }

    pub fn update(&mut self) {
        if self.world.resource::<GameState>().exiting() {
            return;
        }

        if self.world.resource::<GameState>().is_init() {
            self.world.resource_mut::<GameState>().run();
            self.world.run::<PreInit>();
            self.world.run::<Init>();
            self.world.run::<PostInit>();
        }

        self.world.run::<Start>();
        self.world.run::<PreUpdate>();

        let (mut elapsed, fixed_delta) = {
            let time = self.world.resource_mut::<Time>();
            let elapsed = time.fixed_update();
            let fixed_delta = time.fixed_delta_time();

            (elapsed, fixed_delta)
        };

        while elapsed >= fixed_delta {
            self.world.run::<FixedUpdate>();
            elapsed -= fixed_delta;
        }
        self.world.run::<Update>();
        self.world.run::<PostUpdate>();
        self.world.run::<PreRender>();
        self.world.run::<Render>();
        self.world.run::<PostRender>();
        self.world.run::<Finish>();

        for engine in self.engines.iter_mut() {
            engine.extract(&mut self.world);
            engine.update();
        }

        if self.world.resource::<GameState>().exiting() {
            self.world.run::<Shutdown>();
        }

        self.world.resource_mut::<Time>().update();
    }

    pub fn flush_actions<A: Action>(&mut self) {
        self.world.flush_actions::<A>();
    }

    pub fn run(&mut self) {
        let mut game = std::mem::take(self);

        let mut plugins = std::mem::take(&mut game.plugins);
        while !plugins.is_empty() {
            let mut other = Plugins::new();
            plugins.plugins(&mut other);
            game.plugins.extend(&mut plugins);
            plugins = other;
        }

        let mut plugins = std::mem::take(&mut game.plugins);

        plugins.sort();
        plugins.start(&mut game);
        plugins.run(&mut game);
        plugins.finish(&mut game);

        game.world.init();

        let mut runner = game.runner.take().unwrap_or(Box::new(default_runner));
        runner.run(game);
    }
}

pub trait GameRunner: 'static {
    fn run(&mut self, game: Game);
}

impl<F> GameRunner for F
where
    F: FnMut(Game) + 'static,
{
    fn run(&mut self, game: Game) {
        self(game)
    }
}

fn default_runner(mut game: Game) {
    game.update();
}

pub struct PreInit;
impl SchedulePhase for PreInit {
    const PHASE: &'static str = "Game::PreInit";
}

pub struct Init;
impl SchedulePhase for Init {
    const PHASE: &'static str = "Game::Init";
}

pub struct PostInit;
impl SchedulePhase for PostInit {
    const PHASE: &'static str = "Game::PostInit";
}

pub struct Start;
impl SchedulePhase for Start {
    const PHASE: &'static str = "Game::Start";
}

pub struct PreUpdate;
impl SchedulePhase for PreUpdate {
    const PHASE: &'static str = "Game::PreUpdate";
}

pub struct Update;
impl SchedulePhase for Update {
    const PHASE: &'static str = "Game::Update";
}

pub struct PostUpdate;
impl SchedulePhase for PostUpdate {
    const PHASE: &'static str = "Game::PostUpdate";
}

pub struct FixedUpdate;
impl SchedulePhase for FixedUpdate {
    const PHASE: &'static str = "Game::FixedUpdate";
}

pub struct PreRender;
impl SchedulePhase for PreRender {
    const PHASE: &'static str = "Game::PreRender";
}

pub struct Render;
impl SchedulePhase for Render {
    const PHASE: &'static str = "Game::Render";
}

pub struct PostRender;
impl SchedulePhase for PostRender {
    const PHASE: &'static str = "Game::PostRender";
}

pub struct Finish;
impl SchedulePhase for Finish {
    const PHASE: &'static str = "Game::Finish";
}

pub struct Shutdown;
impl SchedulePhase for Shutdown {
    const PHASE: &'static str = "Game::Shutdown";
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GamePhase {
    Init,
    Running,
    Exit,
}

#[derive(Copy, Clone, Debug)]
pub struct GameState {
    phase: GamePhase,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Init,
        }
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    pub(super) fn run(&mut self) {
        self.phase = GamePhase::Running;
    }

    pub fn exit(&mut self) {
        self.phase = GamePhase::Exit;
    }

    pub fn is_init(&self) -> bool {
        self.phase == GamePhase::Init
    }

    pub fn running(&self) -> bool {
        self.phase == GamePhase::Running
    }

    pub fn exiting(&self) -> bool {
        self.phase == GamePhase::Exit
    }
}

impl Resource for GameState {}
