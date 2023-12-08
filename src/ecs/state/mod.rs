use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResetInterval {
    Manual,
    PerFrame,
    PerScene,
}

pub trait State: 'static {
    fn reset(&mut self);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct StateManager {
    states: HashMap<TypeId, (Rc<RefCell<Box<dyn State>>>, ResetInterval)>,
}

impl StateManager {
    pub fn new() -> StateManager {
        StateManager {
            states: HashMap::new(),
        }
    }

    pub fn register<T: State>(&mut self, state: T, interval: ResetInterval) {
        self.states.insert(
            TypeId::of::<T>(),
            (Rc::new(RefCell::new(Box::new(state))), interval),
        );
    }

    pub fn get<'a, T: State>(&'a self) -> Ref<'a, T> {
        let id = TypeId::of::<T>();
        let (state, _) = self.states.get(&id).expect("State not found.");
        let state = state.borrow();

        Ref::map(state, |x| x.as_any().downcast_ref::<T>().unwrap())
    }

    pub fn get_mut<'a, T: State>(&'a self) -> RefMut<'a, T> {
        let id = TypeId::of::<T>();
        let (state, _) = self.states.get(&id).expect("State not found.");
        let state = state.borrow_mut();

        RefMut::map(state, |x| x.as_any_mut().downcast_mut::<T>().unwrap())
    }

    pub fn reset(&mut self, interval: ResetInterval) {
        for (state, state_interval) in self.states.values_mut() {
            if *state_interval == interval && *state_interval != ResetInterval::Manual {
                state.borrow_mut().reset();
            }
        }
    }
}
