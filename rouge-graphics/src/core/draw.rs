use super::ty::color::Color;
use rouge_ecs::macros::Resource;

pub trait Partition<D>: Send + Sync + 'static {
    type Query;

    fn new() -> Self;
    fn insert(&mut self, draw: D);
    fn query(&self, query: Self::Query) -> Vec<&D>;
    fn clear(&mut self);
}

pub trait Draw: Send + Sync + Sized + 'static {
    const PRIORITY: u16 = 0;
    type Partition: Partition<Self>;
    type Render: Render;
}

#[derive(Resource)]
pub struct DrawCalls<D: Draw> {
    calls: D::Partition,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new() -> Self {
        Self {
            calls: D::Partition::new(),
        }
    }

    pub fn insert(&mut self, draw: D) {
        self.calls.insert(draw);
    }

    pub fn query(&self, query: <D::Partition as Partition<D>>::Query) -> Vec<&D> {
        self.calls.query(query)
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl<D: Draw> Partition<D> for Vec<D> {
    type Query = ();

    fn new() -> Self {
        Vec::new()
    }

    fn insert(&mut self, draw: D) {
        self.push(draw);
    }

    fn query(&self, _: Self::Query) -> Vec<&D> {
        self.iter().collect()
    }

    fn clear(&mut self) {
        self.clear();
    }
}

pub trait Render: Send + Sync + 'static {
    fn clear(&self) -> Option<Color>;
    fn depth(&self) -> u32;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[derive(Resource)]
pub struct Renders {
    calls: Vec<Box<dyn Render>>,
}

impl Renders {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn insert(&mut self, render: impl Render) {
        self.calls.push(Box::new(render));
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Render> {
        self.calls.iter().map(|r| &**r)
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}
