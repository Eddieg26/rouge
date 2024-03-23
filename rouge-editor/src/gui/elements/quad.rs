use super::Element;

pub struct Quad {}

impl Quad {
    pub fn new() -> Self {
        Self {}
    }
}

impl Element for Quad {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn draw(&self) {
        println!("Drawing quad");
    }
}
