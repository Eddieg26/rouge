pub mod quad;
pub mod text;

pub trait Element: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    fn draw(&self);
}
