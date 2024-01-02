pub mod compute;
pub mod render;

#[cfg(test)]
mod tests {

    use super::{
        compute::{ComputePassNode, ComputeSubpass, ShaderBindGroup},
        render::{ColorInput, DepthStencilInput, Pass, RenderPassNode, TextureAttachment},
    };
    use crate::graphics::renderer::graph::RenderGraph;

    type OpaquePass = Pass;
    type TransparentPass = Pass;

    #[test]
    pub fn test() {
        let mut render_graph = RenderGraph::new();

        let depth_stencil = render_graph.create_texture(
            "depth-stencil",
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::TextureFormat::Depth16Unorm,
            wgpu::TextureDimension::D2,
        );

        let forward_pass = RenderPassNode::new("forward")
            .with_color(ColorInput {
                color: TextureAttachment::SwapChainImage,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
            .with_depth_stencil(DepthStencilInput {
                depth_stencil: TextureAttachment::Texture(depth_stencil),
                depth_ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                },
                stencil_ops: None,
            })
            .with_sample_count(1)
            .with_pass(OpaquePass::new())
            .with_pass(TransparentPass::new())
            .with_dependency("compute");

        let compute_pass = ComputePassNode::new("compute").with_subpass(
            ComputeSubpass::new()
                .with_bind_group(ShaderBindGroup::new().with_buffer_binding(
                    "buffer",
                    wgpu::BufferBindingType::Storage { read_only: false },
                    false,
                    None,
                    wgpu::ShaderStages::COMPUTE,
                    None,
                ))
                .with_executor(|_ctx, _bind_groups, _pass| {}),
        );

        render_graph.add_node(forward_pass);
        render_graph.add_node(compute_pass);
    }
}
