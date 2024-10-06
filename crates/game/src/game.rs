use crate::app::{App, AppTag, Apps};
use ecs::{
    core::{component::Component, resource::Resource},
    event::Event,
    system::{schedule::Phase, IntoSystemConfigs},
    world::World,
};

pub struct GameBuilder {
    apps: Apps,
    runner: Box<dyn GameRunner>,
}

impl GameBuilder {
    pub fn new() -> Self {
        Self {
            apps: Apps::new(),
            runner: Box::new(default_runner),
        }
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.apps.main().resource::<R>()
    }

    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.apps.main_mut().resource_mut::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.apps.main().non_send_resource::<R>()
    }

    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.apps.main_mut().non_send_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.apps.main().try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.apps.main_mut().try_resource_mut::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.apps.main().try_non_send_resource::<R>()
    }

    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.apps.main_mut().try_non_send_resource_mut::<R>()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.apps.main_mut().register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.apps.main_mut().register_event::<E>();
        self
    }

    pub fn register_resource<R: Resource + Default + Send>(&mut self) -> &mut Self {
        self.apps.main_mut().register_resource::<R>();
        self
    }

    pub fn register_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.apps.main_mut().register_non_send_resource::<R>();
        self
    }

    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.apps.main_mut().add_resource(resource);
        self
    }

    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.apps.main_mut().add_non_send_resource::<R>(resource);
        self
    }

    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.apps.main_mut().remove_resource::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.apps.main_mut().remove_non_send_resource::<R>()
    }

    pub fn invoke_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.apps.main_mut().invoke_event(event);
        self
    }

    pub fn add_sub_app<A: AppTag>(&mut self) -> &mut App {
        self.apps.add::<A>()
    }

    pub fn sub_app<A: AppTag>(&self) -> &App {
        self.apps
            .sub::<A>()
            .expect(&format!("Sub app {:?} not found", A::NAME))
    }

    pub fn sub_app_mut<A: AppTag>(&mut self) -> &mut App {
        self.apps
            .sub_mut::<A>()
            .expect(&format!("Sub app {:?} not found", A::NAME))
    }

    pub fn try_sub_app<A: AppTag>(&self) -> Option<&App> {
        self.apps.sub::<A>()
    }

    pub fn try_sub_app_mut<A: AppTag>(&mut self) -> Option<&mut App> {
        self.apps.sub_mut::<A>()
    }

    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.apps.main_mut().add_systems::<M>(phase, systems);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.apps.main_mut().observe::<E, M>(observers);
        self
    }

    pub fn set_runner(&mut self, runner: impl GameRunner) {
        self.runner = Box::new(runner);
    }

    pub fn run(&mut self) {}
}

pub struct Game {
    world: World,
}

pub trait GameRunner: 'static {
    fn run(&self, game: &mut Game);
}

impl<F: Fn(&mut Game) + 'static> GameRunner for F {
    fn run(&self, game: &mut Game) {
        self(game);
    }
}

fn default_runner(game: &mut Game) {}
