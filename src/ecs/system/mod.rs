use super::world::World;

pub trait System: 'static {
    fn run(&self, world: &World);
}

impl<T: Fn(&World) + 'static> System for T {
    fn run(&self, world: &World) {
        self(world)
    }
}

impl<T: Fn(&World) + 'static> From<T> for Box<dyn System> {
    fn from(f: T) -> Self {
        Box::new(f)
    }
}

