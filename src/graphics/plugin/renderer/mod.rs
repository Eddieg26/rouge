use crate::{
    game::plugin::Plugin,
    graphics::{
        core::{device::RenderDevice, vertex::BaseVertex},
        plugin::GraphicsPlugin,
        resources::{shader::PipelineInfo, GraphicsResources},
    },
};

const VERTEX_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec3<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};
            
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position.xyz, 1.0);
    out.color = vec4<f32>(input.color.xyz, 1.0);
    out.uv = input.uv;
    return out;
}
"#;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 3], color: [f32; 3], uv: [f32; 2]) -> Vertex {
        Vertex {
            position,
            color,
            uv,
        }
    }

    pub fn zero() -> Vertex {
        Vertex {
            position: [0.0; 3],
            color: [0.0; 3],
            uv: [0.0; 2],
        }
    }
}

impl BaseVertex for Vertex {
    fn position(&self) -> glam::Vec3 {
        self.position.into()
    }

    fn attributes(&self) -> Vec<wgpu::VertexAttribute> {
        vec![
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
        ]
    }
}

fn get_2d_pipeline<'a>(
    id: &str,
    vertex: &wgpu::VertexState<'a>,
    blend: wgpu::BlendState,
) -> PipelineInfo<'a> {
    PipelineInfo {
        pipeline_id: id.into(),
        vertex: vertex.clone(),
        depth_stencil: None,
        targets: vec![Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            write_mask: wgpu::ColorWrites::ALL,
            blend: Some(blend),
        })],
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: None,
            unclipped_depth: false,
            conservative: false,
        },
    }
}

fn get_3d_pipeline<'a>(
    id: &str,
    vertex: &wgpu::VertexState<'a>,
    blend: wgpu::BlendState,
) -> PipelineInfo<'a> {
    PipelineInfo {
        pipeline_id: id.into(),
        vertex: vertex.clone(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        targets: vec![Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            write_mask: wgpu::ColorWrites::ALL,
            blend: Some(blend),
        })],
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            conservative: false,
        },
    }
}

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn name(&self) -> &str {
        "renderer-plugin"
    }

    fn run(&self, game: &mut crate::game::Game) {
        let shader = {
            let device = game.world().resource::<RenderDevice>();
            device
                .inner()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(VERTEX_SHADER.into()),
                })
        };

        let attributes = Vertex::zero().attributes();
        let vertex_state = wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }],
        };

        let opaque_blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let transparent_blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let pipelines = vec![
            get_2d_pipeline(
                RenderPipelines::OPAQUE_2D,
                &vertex_state,
                opaque_blend_state,
            ),
            get_3d_pipeline(
                RenderPipelines::OPAQUE_3D,
                &vertex_state,
                opaque_blend_state,
            ),
            get_2d_pipeline(
                RenderPipelines::TRANSPARENT_2D,
                &vertex_state,
                transparent_blend_state,
            ),
            get_3d_pipeline(
                RenderPipelines::TRANSPARENT_3D,
                &vertex_state,
                transparent_blend_state,
            ),
        ];

        let device_res = game.world().resource_ref::<RenderDevice>();
        let device = device_res.get();

        let mut resources = game.world().resource_mut::<GraphicsResources>();
        for pipline in pipelines {
            resources.create_pipelines(device.inner(), &[], &pipline)
        }
    }

    fn dependencies(&self) -> Vec<Box<dyn Plugin>> {
        vec![Box::new(GraphicsPlugin)]
    }
}

pub struct RenderPipelines;

impl RenderPipelines {
    const OPAQUE_2D: &str = "2d_opaque";
    const OPAQUE_3D: &str = "3d_opaque";
    const TRANSPARENT_2D: &str = "2d_transparent";
    const TRANSPARENT_3D: &str = "3d_transparent";
}
