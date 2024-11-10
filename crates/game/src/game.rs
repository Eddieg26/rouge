use crate::{
    app::{AppBuilders, AppTag, Apps, MainApp},
    phases::{Execute, PostExecute, PreExecute, Shutdown, Startup},
    plugin::{Plugin, Plugins},
    MainActions, SubActions, SubApp,
};
use ecs::{
    core::{component::Component, resource::Resource},
    event::{Event, EventId, Events},
    system::{schedule::Phase, IntoSystemConfigs},
    task::TaskPool,
    world::{
        action::{WorldAction, WorldActions},
        World,
    },
};
use std::{error::Error, sync::Arc};

pub struct GameBuilder {
    apps: AppBuilders,
    plugins: Plugins,
    runner: Box<dyn Fn(Game) + 'static>,
}

impl GameBuilder {
    pub fn new() -> Self {
        let mut apps = AppBuilders::new();
        apps.main_world_mut().add_phase::<Startup>();
        apps.main_world_mut().add_phase::<PreExecute>();
        apps.main_world_mut().add_phase::<Execute>();
        apps.main_world_mut().add_phase::<PostExecute>();
        apps.main_world_mut().add_phase::<Shutdown>();
        apps.main_world_mut().register_event::<ExitGame>();

        Self {
            apps,
            plugins: Plugins::new(),
            runner: Box::new(default_runner),
        }
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.apps.main_world().resource::<R>()
    }

    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.apps.main_world_mut().resource_mut::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.apps.main_world().non_send_resource::<R>()
    }

    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.apps.main_world_mut().non_send_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.apps.main_world().try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.apps.main_world_mut().try_resource_mut::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.apps.main_world().try_non_send_resource::<R>()
    }

    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.apps.main_world_mut().try_non_send_resource_mut::<R>()
    }

    pub fn has_resource<R: Resource + Send>(&self) -> bool {
        self.apps.main_world().has_resource::<R>()
    }

    pub fn has_non_send_resource<R: Resource>(&self) -> bool {
        self.apps.main_world().has_non_send_resource::<R>()
    }

    pub fn actions(&self) -> &WorldActions {
        self.apps.main_world().actions()
    }

    pub fn tasks(&self) -> &TaskPool {
        self.apps.main_world().tasks()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.apps.main_world_mut().register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.apps.main_world_mut().register_event::<E>();
        self
    }

    pub fn register_resource<R: Resource + Send>(&mut self) -> &mut Self {
        self.apps.main_world_mut().register_resource::<R>();
        self
    }

    pub fn register_non_send_resource<R: Resource>(&mut self) -> &mut Self {
        self.apps.main_world_mut().register_non_send_resource::<R>();
        self
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        let mut plugins = plugin.dependencies().flatten();
        plugins.add(plugin);
        for (_, plugin) in plugins.iter_mut() {
            plugin.start(self);
        }
        self.plugins.extend(plugins);

        self
    }

    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.apps.main_world_mut().add_resource(resource);
        self
    }

    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.apps
            .main_world_mut()
            .add_non_send_resource::<R>(resource);
        self
    }

    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.apps.main_world_mut().remove_resource::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.apps.main_world_mut().remove_non_send_resource::<R>()
    }

    pub fn scoped_resource<R: Resource + Send>(
        &mut self,
        scope: impl FnOnce(&mut World, &mut R),
    ) -> &mut Self {
        self.apps.main_world_mut().scoped_resource::<R>(scope);
        self
    }

    pub fn scoped_non_send_resource<R: Resource>(
        &mut self,
        scope: impl FnOnce(&mut World, &mut R),
    ) -> &mut Self {
        self.apps
            .main_world_mut()
            .scoped_non_send_resource::<R>(scope);
        self
    }

    pub fn invoke_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.apps.main_world_mut().invoke_event(event);
        self
    }

    pub fn add_sub_app<A: AppTag>(&mut self) -> &mut SubApp {
        let sub_actions = {
            let actions = self.actions().clone();
            let app = self.apps.add::<A>();
            app.add_resource(MainActions::new(actions));
            app.actions().clone()
        };

        self.add_resource(SubActions::<A>::new(sub_actions));
        self.apps.sub_mut::<A>().unwrap()
    }

    pub fn sub_app<A: AppTag>(&self) -> &SubApp {
        self.apps
            .sub::<A>()
            .expect(&format!("Sub app {:?} not found", A::NAME))
    }

    pub fn sub_app_mut<A: AppTag>(&mut self) -> &mut SubApp {
        self.apps
            .sub_mut::<A>()
            .expect(&format!("Sub app {:?} not found", A::NAME))
    }

    pub fn try_sub_app<A: AppTag>(&self) -> Option<&SubApp> {
        self.apps.sub::<A>()
    }

    pub fn try_sub_app_mut<A: AppTag>(&mut self) -> Option<&mut SubApp> {
        self.apps.sub_mut::<A>()
    }

    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.apps.main_world_mut().add_systems::<M>(phase, systems);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.apps.main_world_mut().observe::<E, M>(observers);
        self
    }

    pub fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.apps.main_world_mut().add_phase::<P>();
        self
    }

    pub fn add_sub_phase<Main: Phase, Sub: Phase>(&mut self) -> &mut Self {
        self.apps.main_world_mut().add_sub_phase::<Main, Sub>();
        self
    }

    pub fn add_phase_before<P: Phase, Before: Phase>(&mut self) -> &mut Self {
        self.apps.main_world_mut().add_phase_before::<P, Before>();
        self
    }

    pub fn add_phase_after<P: Phase, After: Phase>(&mut self) -> &mut Self {
        self.apps.main_world_mut().add_phase_after::<P, After>();
        self
    }

    pub fn set_runner(&mut self, runner: impl Fn(Game) + 'static) -> &mut Self {
        self.runner = Box::new(runner);
        self
    }

    pub fn run(&mut self) {
        let mut plugins = std::mem::take(&mut self.plugins);
        plugins.run(self);
        plugins.finish(self);

        self.add_resource(TaskPool::default());
        let apps = self.apps.into_apps();
        let game = Game { apps };
        (self.runner)(game);
    }
}

pub struct Game {
    apps: Apps,
}

impl Game {
    pub fn new() -> GameBuilder {
        GameBuilder::new()
    }

    pub fn app(&self) -> &MainApp {
        self.apps.main_app()
    }

    pub fn app_mut(&mut self) -> &mut MainApp {
        self.apps.main_app_mut()
    }

    pub fn startup(&mut self) {
        self.apps.main_app_mut().run(Startup);
    }

    pub fn update(&mut self) -> Option<ExitGame> {
        self.apps.main_app_mut().run(PreExecute);
        self.apps.main_app_mut().run(Execute);
        self.apps.run();
        self.apps.main_app_mut().run(PostExecute);
        let events = self
            .apps
            .main_world_mut()
            .resource_mut::<Events<ExitGame>>();
        events.drain().last()
    }

    pub fn shutdown(&mut self) {
        self.apps.main_app_mut().run(Shutdown);
    }

    pub fn flush(&mut self) {
        self.apps.main_app_mut().world_mut().flush(None);
    }

    pub fn flush_type<E: Event>(&mut self) {
        self.apps
            .main_app_mut()
            .world_mut()
            .flush_type(EventId::of::<E>());
    }
}

fn default_runner(mut game: Game) {
    game.startup();
    game.update();
    game.shutdown();
}

#[derive(Debug, Clone)]
pub enum ExitGame {
    Success,
    Failure(Arc<dyn Error + Send + Sync + 'static>),
}

impl ExitGame {
    pub fn success() -> Self {
        ExitGame::Success
    }

    pub fn failure<E: Error + Send + Sync + 'static>(error: E) -> Self {
        ExitGame::Failure(Arc::new(error))
    }

    pub fn is_success(&self) -> bool {
        matches!(self, ExitGame::Success)
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

impl WorldAction for ExitGame {
    fn execute(self, world: &mut World) -> Option<()> {
        Some(world.resource_mut::<Events<ExitGame>>().add(self))
    }
}

impl Event for ExitGame {}
