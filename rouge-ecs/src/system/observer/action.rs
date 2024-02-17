use crate::{
    storage::{blob::Blob, sparse::SparseMap},
    system::SystemArg,
    world::{resource::Resource, World},
};
use std::{
    any::TypeId,
    sync::{Arc, Mutex},
    vec,
};

pub trait Action: 'static {
    type Output;
    const PRIORITY: u32 = 0;

    fn execute(&mut self, world: &mut World) -> Self::Output;

    fn skip(&self, _: &World) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct ActionReflector {
    priority: u32,
    execute: fn(&mut World, &Blob, &mut ActionOutputs),
}

impl ActionReflector {
    pub fn new<A: Action>() -> Self {
        Self {
            priority: A::PRIORITY,
            execute: |world, blob, outputs| {
                let action = blob.get_mut::<A>(0).expect("Action not found");
                outputs.add::<A>(action.execute(world));
            },
        }
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn execute(&self, world: &mut World, blob: &Blob, outputs: &mut ActionOutputs) {
        (self.execute)(world, blob, outputs);
    }
}

#[derive(Default, Clone)]
pub struct ActionReflectors {
    reflectors: SparseMap<TypeId, ActionReflector>,
}

impl ActionReflectors {
    pub fn new() -> Self {
        Self {
            reflectors: SparseMap::new(),
        }
    }

    pub fn register<A: Action>(&mut self) {
        let type_id = TypeId::of::<A>();
        if !self.reflectors.contains(&type_id) {
            self.reflectors.insert(type_id, ActionReflector::new::<A>());
        }

        self.sort();
    }

    pub fn sort(&mut self) {
        self.reflectors.sort(|a, b| a.priority().cmp(&b.priority()));
    }

    pub fn execute(&self, world: &mut World) -> ActionOutputs {
        let mut outputs = ActionOutputs::new();

        for (type_id, blob) in world.actions_mut().drain().drain(..) {
            let reflector = self
                .reflectors
                .get(&type_id)
                .expect("Action not registered");
            reflector.execute(world, &blob, &mut outputs);
        }

        outputs
    }

    pub fn execute_actions<A: Action>(&self, world: &mut World) -> ActionOutputs {
        let mut outputs = ActionOutputs::new();

        let type_id = TypeId::of::<A>();
        let reflector = self
            .reflectors
            .get(&type_id)
            .expect("Action not registered");
        let actions = world.actions_mut().filter(&type_id);
        for blob in actions {
            reflector.execute(world, &blob, &mut outputs);
        }

        outputs
    }
}

#[derive(Default, Clone)]
pub struct Actions {
    actions: Arc<Mutex<Vec<(TypeId, Blob)>>>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn drain(&mut self) -> Vec<(TypeId, Blob)> {
        let mut actions = self.actions.lock().unwrap();
        std::mem::take(&mut *actions)
    }

    pub fn add<A: Action>(&mut self, action: A) {
        let type_id = TypeId::of::<A>();
        let mut actions = self.actions.lock().unwrap();
        let mut data = Blob::new::<A>();
        data.push(action);
        actions.push((type_id, data));
    }

    pub fn pop(&mut self) -> Option<(TypeId, Blob)> {
        let mut actions = self.actions.lock().unwrap();
        actions.pop()
    }

    pub fn filter(&mut self, type_id: &TypeId) -> Vec<Blob> {
        let mut actions = self.actions.lock().unwrap();
        let mut filtered = Vec::new();
        let mut index = 0;
        while index < actions.len() {
            if actions[index].0 == *type_id {
                filtered.push(actions.remove(index).1);
            } else {
                index += 1;
            }
        }

        filtered
    }

    pub fn is_empty(&self) -> bool {
        let actions = self.actions.lock().unwrap();
        actions.is_empty()
    }

    pub fn clear(&mut self) {
        let mut actions = self.actions.lock().unwrap();
        actions.clear();
    }
}

impl SystemArg for Actions {
    type Item<'a> = Actions;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.actions().clone()
    }

    fn metas() -> Vec<crate::world::meta::AccessMeta> {
        vec![]
    }
}

pub struct ActionOutputs {
    outputs: SparseMap<TypeId, Blob>,
}

impl ActionOutputs {
    pub(crate) fn new() -> Self {
        Self {
            outputs: SparseMap::new(),
        }
    }

    pub fn take(&mut self) -> Self {
        let mut outputs = Self::new();
        std::mem::swap(&mut outputs, self);
        outputs
    }

    pub fn add<A: Action>(&mut self, output: A::Output) {
        if let Some(outputs) = self.outputs.get_mut(&TypeId::of::<A>()) {
            outputs.push(output);
        } else {
            let mut outputs = Blob::new::<A::Output>();
            outputs.push(output);
            self.outputs.insert(TypeId::of::<A>(), outputs);
        }
    }

    pub fn merge(&mut self, mut outputs: Self) {
        for (type_id, mut blob) in outputs.outputs.drain() {
            if let Some(outputs) = self.outputs.get_mut(&type_id) {
                outputs.append(&mut blob);
            } else {
                self.outputs.insert(type_id, blob);
            }
        }
    }

    pub fn contains(&self, type_id: &TypeId) -> bool {
        self.outputs.contains(type_id)
    }

    pub fn keys(&self) -> impl Iterator<Item = &TypeId> {
        self.outputs.keys()
    }

    pub fn remove(&mut self, type_id: &TypeId) -> Option<Blob> {
        self.outputs.remove(type_id)
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }
}

impl Resource for ActionReflectors {}
impl Resource for ActionOutputs {}
