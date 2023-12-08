use crate::graphics::{
    renderer::{context::RenderContext, graph::RenderGraphNode, NodeId},
    resources::TextureId,
};

pub enum TextureAttachment {
    SwapChainImage,
    Texture(TextureId),
}

pub struct ColorInput {
    pub color: TextureAttachment,
    pub resolve_target: Option<TextureAttachment>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

pub struct DepthStencilInput {
    pub depth_stencil: TextureAttachment,
    pub depth_ops: wgpu::Operations<f32>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

pub struct RenderPassNodeInfo {
    pub colors: Vec<ColorInput>,
    pub depth_stencil: Option<DepthStencilInput>,
    pub sample_count: u32,
}

impl RenderPassNodeInfo {
    pub fn new() -> RenderPassNodeInfo {
        RenderPassNodeInfo {
            colors: Vec::new(),
            depth_stencil: None,
            sample_count: 1,
        }
    }
}

pub trait RenderGroup: 'static {
    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::RenderPass);
}

impl<T: Fn(&RenderContext, &mut wgpu::RenderPass) + 'static> RenderGroup for T {
    fn execute(&self, ctx: &RenderContext, render_pass: &mut wgpu::RenderPass) {
        self(ctx, render_pass);
    }
}

pub struct Pass {
    groups: Vec<Box<dyn RenderGroup>>,
}

impl Pass {
    pub fn new() -> Pass {
        Pass { groups: Vec::new() }
    }

    pub fn with_group(mut self, group: impl RenderGroup) -> Pass {
        self.groups.push(Box::new(group));
        self
    }

    pub fn execute(&self, ctx: &RenderContext, render_pass: &mut wgpu::RenderPass) {
        for group in &self.groups {
            group.execute(ctx, render_pass);
        }
    }
}

pub struct RenderPassNode {
    id: NodeId,
    dependencies: Vec<NodeId>,
    info: RenderPassNodeInfo,
    passes: Vec<Pass>,
}

impl RenderPassNode {
    pub fn new(id: impl Into<NodeId>) -> RenderPassNode {
        RenderPassNode {
            id: id.into(),
            dependencies: Vec::new(),
            info: RenderPassNodeInfo::new(),
            passes: Vec::new(),
        }
    }

    pub fn with_pass(mut self, pass: Pass) -> RenderPassNode {
        self.passes.push(pass);
        self
    }

    pub fn with_color(mut self, color: ColorInput) -> RenderPassNode {
        self.info.colors.push(color);
        self
    }

    pub fn with_colors(mut self, colors: Vec<ColorInput>) -> RenderPassNode {
        self.info.colors = colors;
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: DepthStencilInput) -> RenderPassNode {
        self.info.depth_stencil = Some(depth_stencil);
        self
    }

    pub fn with_sample_count(mut self, sample_count: u32) -> RenderPassNode {
        self.info.sample_count = sample_count;
        self
    }

    pub fn with_dependency(mut self, dependency: impl Into<NodeId>) -> RenderPassNode {
        self.dependencies.push(dependency.into());
        self
    }

    fn get_color_attachments<'a>(
        &'a self,
        ctx: &'a RenderContext,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment>> {
        let mut attachments = Vec::new();

        for color in &self.info.colors {
            let attachment = match &color.color {
                TextureAttachment::SwapChainImage => Some(wgpu::RenderPassColorAttachment {
                    view: ctx.render_target(),
                    resolve_target: color.resolve_target.as_ref().map(|t| match t {
                        TextureAttachment::SwapChainImage => ctx.render_target(),
                        TextureAttachment::Texture(id) => ctx
                            .dyn_texture(id)
                            .expect("Texture attachement not found.")
                            .view(),
                    }),
                    ops: color.ops,
                }),
                TextureAttachment::Texture(id) => Some(wgpu::RenderPassColorAttachment {
                    view: ctx.dyn_texture(id).unwrap().view(),
                    resolve_target: color.resolve_target.as_ref().map(|t| match t {
                        TextureAttachment::SwapChainImage => ctx.render_target(),
                        TextureAttachment::Texture(id) => ctx
                            .dyn_texture(id)
                            .expect("Texture attachement not found.")
                            .view(),
                    }),
                    ops: color.ops,
                }),
            };

            attachments.push(attachment);
        }

        attachments
    }

    fn get_depth_stencil_attachment<'a>(
        &'a self,
        ctx: &'a RenderContext,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        self.info.depth_stencil.as_ref().map(|ds| {
            let depth_stencil = match &ds.depth_stencil {
                TextureAttachment::SwapChainImage => ctx.render_target(),
                TextureAttachment::Texture(id) => ctx
                    .dyn_texture(id)
                    .expect("Texture attachement not found.")
                    .view(),
            };

            wgpu::RenderPassDepthStencilAttachment {
                view: depth_stencil,
                depth_ops: Some(ds.depth_ops),
                stencil_ops: ds.stencil_ops,
            }
        })
    }
}

impl RenderGraphNode for RenderPassNode {
    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &self.get_color_attachments(ctx),
            depth_stencil_attachment: self.get_depth_stencil_attachment(ctx),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        for pass in &self.passes {
            pass.execute(ctx, &mut render_pass);
        }
    }

    fn id(&self) -> &NodeId {
        &self.id
    }

    fn dependencies(&self) -> &Vec<NodeId> {
        &self.dependencies
    }

    fn dependencies_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.dependencies
    }
}
