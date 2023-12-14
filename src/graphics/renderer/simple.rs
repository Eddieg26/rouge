use std::borrow::Cow;
use winit::window::Window;

use crate::{
    ecs::{Resource, World},
    game::states::GamePhase,
    graphics::{
        core::{device::RenderDevice, surface::RenderSurface},
        plugin::window::{events::WindowResized, WindowPlugin},
        resources::{
            buffer::Buffer,
            texture::{Texture, Texture2d, TextureInfo},
        },
    },
};

#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[repr(C)]
pub struct SimpleVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub tex_coords: [f32; 2],
}

pub struct SimpleRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl SimpleRenderer {
    pub fn new(device: &RenderDevice, surface: &RenderSurface) -> Self {
        let shader_source = wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            " 
        struct VertexInput {
            @location(0) position: vec3<f32>,
            @location(1) color: vec3<f32>,
            @location(2) tex_coords: vec2<f32>,
        };

        struct VertexOutput {
            @builtin(position) position: vec4<f32>,
            @location(0) color: vec4<f32>,
            @location(1) tex_coords: vec2<f32>,
        }
            
        @vertex
        fn vs_main(input: VertexInput) -> VertexOutput {
            var out: VertexOutput;
            out.position = vec4<f32>(input.position.xyz, 1.0);
            out.color = vec4<f32>(input.color.xyz, 1.0);
            out.tex_coords = input.tex_coords;
            return out;
        }

        @group(0) @binding(0)
        var texture: texture_2d<f32>;

        @group(0) @binding(1)
        var tex_sampler: sampler;

        @fragment
         fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
            var color = vec4<f32>(input.color * textureSample(texture, tex_sampler, input.tex_coords.xy));
            return color;
        }
        ",
        ));

        let shader = device
            .inner()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("simple-shader"),
                source: shader_source,
            });

        let bind_group_layout =
            device
                .inner()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("simple-bind-group-layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            device
                .inner()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("simple-pipeline-layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .inner()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("simple-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 32,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface.format(),
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        let texture =
            Texture2d::from_info(device.inner(), device.queue(), &TextureInfo::red(256, 256));
        let bind_group = device
            .inner()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("simple-bind-group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(texture.sampler()),
                    },
                ],
            });

        let vertices = [
            SimpleVertex {
                position: [-0.5, 0.5, 0.0],
                color: [1.0, 0.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
            SimpleVertex {
                position: [-0.5, -0.5, 0.0],
                color: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            SimpleVertex {
                position: [0.5, -0.5, 0.0],
                color: [0.0, 0.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
            SimpleVertex {
                position: [0.5, 0.5, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
        ];

        let vertex_buffer =
            Buffer::from_data(device.inner(), wgpu::BufferUsages::VERTEX, &vertices);

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = Buffer::from_data(device.inner(), wgpu::BufferUsages::INDEX, &indices);

        Self {
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render(&self, device: &RenderDevice, surface: &RenderSurface) -> Result<(), String> {
        let current_texture = surface.current_texture().map_err(|e| e.to_string())?;
        let view = current_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .inner()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("simple-render-encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("simple-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                view: &view,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.inner().slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.inner().slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);

        drop(render_pass);

        device.queue().submit(Some(encoder.finish()));

        current_texture.present();

        Ok(())
    }
}

impl Resource for SimpleRenderer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct SimpleRendererPlugin;

impl crate::game::plugin::Plugin for SimpleRendererPlugin {
    fn name(&self) -> &str {
        "simple-renderer-plugin"
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

        game.add_resource(surface);
        game.add_resource(device);
        game.observe::<WindowResized>(|sizes, world| {
            let event = sizes.last().unwrap();
            if event.size.width == 0 || event.size.height == 0 {
                return;
            }
            let surface = world.resource::<RenderSurface>();
            let device = world.resource::<RenderDevice>();

            surface.configure(device.inner(), event.size);
        });
    }

    fn run(&self, game: &mut crate::game::Game) {
        let renderer = {
            let surface = game.world().resource::<RenderSurface>();
            let device = game.world().resource::<RenderDevice>();

            SimpleRenderer::new(&device, &surface)
        };

        game.add_resource(renderer);
        game.add_phase_system(GamePhase::Render, |world: &World| {
            let renderer = world.resource::<SimpleRenderer>();
            let surface = world.resource::<RenderSurface>();
            let device = world.resource::<RenderDevice>();

            renderer.render(&device, &surface).unwrap();
        });
    }

    fn dependencies(&self) -> Vec<Box<dyn crate::game::plugin::Plugin>> {
        vec![Box::new(WindowPlugin::default())]
    }
}
