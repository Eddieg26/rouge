use ecs::{
    core::{component::Component, resource::Resource, IndexMap, Type},
    event::Event,
    system::{schedule::Phase, IntoSystemConfigs},
    world::World,
};

pub trait AppTag: 'static {
    const NAME: &'static str;
}

pub struct App {
    world: World,
}

impl App {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.world.resource_mut::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.world.non_send_resource::<R>()
    }

    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.world.non_send_resource_mut::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.world.try_resource_mut::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_non_send_resource::<R>()
    }

    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.world.try_non_send_resource_mut::<R>()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.world.register::<C>();
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn register_resource<R: Resource + Default + Send>(&mut self) -> &mut Self {
        self.world.register_resource::<R>();
        self
    }

    pub fn register_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.world.register_non_send_resource::<R>();
        self
    }

    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.world.add_resource(resource);
        self
    }

    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.add_non_send_resource::<R>(resource);
        self
    }

    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.world.remove_resource::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.world.remove_non_send_resource::<R>()
    }

    pub fn invoke_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.world.invoke_event(event);
        self
    }

    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.world.add_systems::<M>(phase, systems);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.world.observe::<E, M>(observers);
        self
    }
}

pub struct Apps {
    main: App,
    sub: IndexMap<Type, App>,
}

impl Apps {
    pub fn new() -> Self {
        Self {
            main: App::new(),
            sub: IndexMap::new(),
        }
    }

    pub fn main(&self) -> &App {
        &self.main
    }

    pub fn main_mut(&mut self) -> &mut App {
        &mut self.main
    }

    pub fn sub<A: AppTag>(&self) -> Option<&App> {
        self.sub.get(&Type::of::<A>())
    }

    pub fn sub_mut<A: AppTag>(&mut self) -> Option<&mut App> {
        self.sub.get_mut(&Type::of::<A>())
    }

    pub fn sub_dyn(&self, tag: Type) -> Option<&App> {
        self.sub.get(&tag)
    }

    pub fn sub_dyn_mut(&mut self, tag: Type) -> Option<&mut App> {
        self.sub.get_mut(&tag)
    }

    pub fn add<A: AppTag>(&mut self) -> &mut App {
        let ty = Type::of::<A>();
        if !self.sub.contains_key(&ty) {
            self.sub.insert(ty, App::new());
        }

        self.sub.get_mut(&ty).unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = &App> {
        self.sub.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut App> {
        self.sub.values_mut()
    }
}
