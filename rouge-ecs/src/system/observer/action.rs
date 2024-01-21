use crate::{
    storage::{blob::Blob, sparse::SparseMap},
    world::{resource::Resource, World},
};
use std::any::TypeId;

pub struct ActionData {
    actions: Blob,
    priority: u32,
    execute: Box<dyn Fn(&mut World, &mut Blob, &mut ActionOutputs) + Send + Sync>,
}

impl ActionData {
    pub fn new<A: Action>() -> Self {
        Self {
            actions: Blob::new::<A>(),
            priority: A::PRIORITY,
            execute: Box::new(|world, blob, outputs| {
                for action in blob.iter_mut::<A>() {
                    outputs.add::<A>(action.execute(world));
                }
            }),
        }
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn execute(&self, world: &mut World, blob: &mut Blob, outputs: &mut ActionOutputs) {
        (self.execute)(world, blob, outputs);
    }

    pub fn actions(&self) -> &Blob {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut Blob {
        &mut self.actions
    }

    pub fn clear(&mut self) -> Blob {
        self.actions.take()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

pub trait Action: 'static {
    type Output;
    const PRIORITY: u32 = 0;

    fn execute(&mut self, world: &mut World) -> Self::Output;

    fn skip(&self, _: &World) -> bool {
        false
    }
}

#[derive(Default)]
pub struct Actions {
    actions: SparseMap<TypeId, ActionData>,
}

impl Actions {
    pub fn new() -> Self {
        Self {
            actions: SparseMap::new(),
        }
    }

    pub fn add<A: Action>(&mut self, action: A) {
        let type_id = TypeId::of::<A>();
        if let Some(data) = self.actions.get_mut(&type_id) {
            data.actions.push(action);
        } else {
            let mut data = ActionData::new::<A>();
            data.actions.push(action);
            self.actions.insert(type_id, data);
        }
    }

    pub fn append(&mut self, mut actions: Actions) {
        for (type_id, mut data) in actions.actions.drain() {
            if let Some(other) = self.actions.get_mut(&type_id) {
                other.actions.append(&mut data.actions);
            } else {
                self.actions.insert(type_id, data);
            }
        }
    }

    fn sort(&mut self) {
        self.actions.sort(|a, b| a.priority().cmp(&b.priority()));
    }

    pub fn execute(&mut self, world: &mut World) -> ActionOutputs {
        self.sort();
        let mut outputs = ActionOutputs::new();

        for data in self.actions.values_mut() {
            let mut actions = data.clear();
            data.execute(world, &mut actions, &mut outputs);
        }

        outputs
    }

    pub fn is_empty(&self) -> bool {
        self.actions.values().iter().all(|data| data.is_empty())
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

impl Resource for Actions {}
impl Resource for ActionOutputs {}
