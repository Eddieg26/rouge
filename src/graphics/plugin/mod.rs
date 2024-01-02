use self::window::events::WindowResized;
use super::{
    core::{device::RenderDevice, surface::RenderSurface},
    renderer::graph::RenderGraph,
};
use crate::game::plugin::Plugin;
use winit::window::Window;

pub mod material;
pub mod renderer;
pub mod window;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn name(&self) -> &str {
        "graphics-plugin"
    }

    fn start(&self, game: &mut crate::game::Game) {
        let surface = {
            let window = game.world().resource::<Window>();
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });

            RenderSurface::new(&window, &instance)
        };

        let device = RenderDevice::new(surface.adapter());

        game.add_resource(RenderGraph::new());
        game.add_resource(surface);
        game.add_resource(device);
        game.observe::<WindowResized>(|sizes, world| {
            let event = sizes.last().unwrap();
            if event.size.width == 0 || event.size.height == 0 {
                return;
            }
            let mut surface = world.resource_mut::<RenderSurface>();
            let device = world.resource::<RenderDevice>();

            surface.resize(device.inner(), event.size);
        });
    }

    fn finish(&self, game: &mut crate::game::Game) {
        let device = game.world().resource::<RenderDevice>();
        let surface = game.world().resource::<RenderSurface>();

        game.world().resource_mut::<RenderGraph>().build(
            &device,
            surface.width(),
            surface.height(),
        );
    }

    fn dependencies(&self) -> Vec<Box<dyn Plugin>> {
        vec![Box::new(window::WindowPlugin::default())]
    }
}
