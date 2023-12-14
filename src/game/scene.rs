use super::schedule::SchedulePlan;
use crate::ecs::Resource;
use std::{any::TypeId, collections::HashMap, rc::Rc};

pub trait Scene: 'static {
    fn name(&self) -> &str;
    fn start(&self, world: &crate::ecs::World);
    fn end(&self, world: &crate::ecs::World);
    fn plan(&self) -> SchedulePlan;
}

pub struct SceneManager {
    scenes: HashMap<TypeId, Rc<Box<dyn Scene>>>,
    start_scene: Option<TypeId>,
    current: Option<Rc<Box<dyn Scene>>>,
    next: Option<Rc<Box<dyn Scene>>>,
    is_quitting: bool,
}

impl SceneManager {
    pub fn new() -> SceneManager {
        SceneManager {
            scenes: HashMap::new(),
            current: None,
            next: None,
            start_scene: None,
            is_quitting: false,
        }
    }

    pub fn set_start<T: Scene>(&mut self) -> &mut Self {
        let scene = TypeId::of::<T>();
        self.start_scene = Some(scene);

        self
    }

    pub fn add_scene<T: Scene>(&mut self, scene: T) -> &mut Self {
        let id = TypeId::of::<T>();
        self.scenes.insert(id, Rc::new(Box::new(scene)));

        self
    }

    pub fn quit(&mut self) -> &mut Self {
        self.is_quitting = true;

        self
    }

    pub fn switch_to<T: Scene>(&mut self) -> &mut Self {
        if self.is_quitting {
            return self;
        }

        let id = TypeId::of::<T>();
        self.next = self.scenes.get(&id).cloned();

        self
    }

    pub fn quitting(&self) -> bool {
        self.is_quitting
    }

    pub(super) fn next(&self) -> Option<&dyn Scene> {
        self.next.as_ref().map(|scene| scene.as_ref().as_ref())
    }

    pub(super) fn current(&self) -> Option<&dyn Scene> {
        self.current.as_ref().map(|scene| scene.as_ref().as_ref())
    }

    pub(super) fn start(&mut self) {
        if self.is_quitting {
            return;
        }

        let scene = self
            .start_scene
            .and_then(|id| self.scenes.get(&id).cloned());

        if let Some(scene) = scene {
            self.current = Some(scene);
        }
    }

    pub(super) fn transition(&mut self) {
        if self.is_quitting {
            return;
        }

        self.current = self.next.take();
    }
}

impl Resource for SceneManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
