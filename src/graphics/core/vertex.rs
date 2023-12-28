pub trait BaseVertex: bytemuck::Pod + bytemuck::Zeroable {
    fn position(&self) -> glam::Vec3;
    fn attributes() -> Vec<wgpu::VertexAttribute>;
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub normal: glam::Vec3,
    pub uv: glam::Vec2,
}

impl Vertex {
    pub fn new(position: glam::Vec3, normal: glam::Vec3, uv: glam::Vec2) -> Vertex {
        Vertex {
            position,
            normal,
            uv,
        }
    }

    pub fn zero() -> Vertex {
        Vertex {
            position: glam::Vec3::ZERO,
            normal: glam::Vec3::ZERO,
            uv: glam::Vec2::ZERO,
        }
    }
}

impl BaseVertex for Vertex {
    fn position(&self) -> glam::Vec3 {
        self.position
    }

    fn attributes() -> Vec<wgpu::VertexAttribute> {
        vec![
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<glam::Vec3>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: (std::mem::size_of::<glam::Vec3>() * 2) as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
        ]
    }
}
