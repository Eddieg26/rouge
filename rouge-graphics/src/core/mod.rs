use rouge_core::math;


pub mod device;
pub mod surface;
pub mod ty;

pub trait BaseVertex: bytemuck::Pod + bytemuck::Zeroable {
    fn position(&self) -> math::Vec3;
    fn attributes() -> Vec<wgpu::VertexAttribute>;
}

