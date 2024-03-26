use rouge_ecs::macros::Resource;

pub trait Partition<D: Draw>: Send + Sync + 'static {
    type Query;

    fn new() -> Self;
    fn insert(&mut self, draw: D);
    fn query(&self, query: Self::Query) -> Vec<&D>;
    fn clear(&mut self);
}

pub trait Draw: Send + Sync + Sized + 'static {
    type Partition: Partition<Self>;
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
