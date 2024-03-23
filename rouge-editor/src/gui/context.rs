use std::{collections::HashMap, sync::{Arc, Mutex}};

use super::core::ElementId;

pub struct Context {
    states: HashMap<ElementId, Box<dyn BaseUI>>
}

impl Context {
    pub fn new() -> Self {
        Self {
            states: HashMap::new()
        }
    }

    pub fn use_state<S: Send + Sync + PartialEq + 'static>(&mut self, id: ElementId, state: S) -> UIState<S> {
        if let Some(state) = self.states.get(&id) {
            if let Some(state) = state.as_any().downcast_ref::<UIState<S>>() {
                return state.clone();
            }
        }

        let state = UIState::new(state);
        self.states.insert(id, Box::new(state.clone()));
        state
    }
}

pub trait BaseUI: 'static + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}


pub enum UpdateMode<S: Send + Sync + PartialEq + 'static> {
    Set(S),
    Update(Box<dyn Fn(&S) -> S + Send + Sync>),
}

pub struct UIState<S: Send + Sync + PartialEq + 'static> {
    state: Arc<S>,
    update: Arc<Mutex<Option<UpdateMode<S>>>>,
}

impl<S: Send + Sync + PartialEq> UIState<S> {
    pub fn new(state: S) -> Self {
        Self {
            state: Arc::new(state),
            update: Arc::new(Mutex::new(None)),
        }
    }

    pub fn value(&self) -> &S {
        &self.state
    }

    pub fn set(&mut self, state: S) {
        self.update
            .lock()
            .unwrap()
            .replace(UpdateMode::Set(state));
    }

    pub fn update(&mut self, update: impl Fn(&S) -> S + Send + Sync + 'static) {
        self.update
            .lock()
            .unwrap()
            .replace(UpdateMode::Update(Box::new(update)));
    }

    /// Apply the update to the state and return true if the state has changed.
    pub(crate) fn apply(&mut self) -> bool {
        let mut update = self.update.lock().unwrap();
        match update.take() {
            Some(UpdateMode::Set(state)) => {
                let new = Arc::new(state);
                let old = std::mem::replace(&mut self.state, new);
                self.state != old
            }
            Some(UpdateMode::Update(update)) => {
                let new = Arc::new(update(&*self.state));
                let old = std::mem::replace(&mut self.state, new);
                self.state != old
            }
            None => false,
        }
    }
}

impl<S: Send + Sync + PartialEq + 'static> Clone for UIState<S> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            update: self.update.clone(),
        }
    }
}

impl<S: Send + Sync + PartialEq + 'static> BaseUI for UIState<S> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
