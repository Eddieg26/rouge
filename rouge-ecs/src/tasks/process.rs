use crate::{observer::Action, World};

pub trait Process: Send + Sync + 'static {
    fn init(&mut self, main: &mut World, world: &mut World);
    fn execute(&mut self, world: &mut World);
}

impl Process for () {
    fn init(&mut self, _: &mut World, _: &mut World) {}
    fn execute(&mut self, _: &mut World) {}
}

pub struct SubProcess {
    world: Option<World>,
    process: Box<dyn Process>,
}

impl SubProcess {
    pub fn new<P: Process>(process: P) -> Self {
        Self {
            world: None,
            process: Box::new(process),
        }
    }

    pub fn init(&mut self, main: &mut World) {
        if self.world.is_some() {
            return;
        } else {
            self.world = Some(World::new_sub(main));
            let world = self.world.as_mut().unwrap();
            self.process.init(main, world);
            world.init();
        }
    }

    pub fn execute(&mut self) {
        let world = self.world.as_mut().expect("World not initialized");
        self.process.execute(world);
    }
}

impl Default for SubProcess {
    fn default() -> Self {
        Self::new(())
    }
}

pub struct StartProcess {
    process: SubProcess,
}

impl StartProcess {
    pub fn new<P: Process>(process: P) -> Self {
        Self {
            process: SubProcess::new(process),
        }
    }
}

impl Action for StartProcess {
    type Output = ();

    fn execute(&mut self, world: &mut World) -> Self::Output {
        let mut process = std::mem::take(&mut self.process);
        process.init(world);
        std::thread::spawn(move || {
            process.execute();
        });
    }
}
