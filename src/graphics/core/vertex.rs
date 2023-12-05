pub trait BaseVertex: bytemuck::Pod + bytemuck::Zeroable {
    fn position(&self) -> glam::Vec3;
    fn attributes(&self) -> Vec<wgpu::VertexAttribute>;
}
