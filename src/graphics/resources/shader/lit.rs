use super::ShaderTemplate;

pub struct LitShaderTemplate;

impl ShaderTemplate for LitShaderTemplate {
    const MATERIAL_BIND_GROUP: u32 = 1;
    fn create_shader(
        _device: &wgpu::Device,
        _material: &crate::graphics::resources::material::Material,
    ) -> super::Shader {
        todo!()
    }
}
