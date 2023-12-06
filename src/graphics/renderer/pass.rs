use std::rc::Rc;

use super::{context::RenderContext, NodeId, RenderGraph, RenderGraphNode};
use crate::graphics::{
    core::gpu::GpuInstance,
    resources::{texture::TextureInfo, TextureId},
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

pub trait RenderGroup {
    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::RenderPass);
}

pub struct Pass {
    groups: Vec<Box<dyn RenderGroup>>,
}

impl Pass {
    pub fn new() -> Pass {
        Pass { groups: Vec::new() }
    }

    pub fn add_group(&mut self, group: Box<dyn RenderGroup>) {
        self.groups.push(group);
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

    fn inputs(&self) -> Vec<super::Attachment> {
        let mut inputs = self
            .info
            .colors
            .iter()
            .map(|c| match &c.color {
                TextureAttachment::SwapChainImage => super::Attachment::SwapChainImage,
                TextureAttachment::Texture(id) => super::Attachment::Texture(*id),
            })
            .collect::<Vec<_>>();

        if let Some(ds) = &self.info.depth_stencil {
            inputs.push(match &ds.depth_stencil {
                TextureAttachment::SwapChainImage => super::Attachment::SwapChainImage,
                TextureAttachment::Texture(id) => super::Attachment::Texture(*id),
            });
        };

        inputs
    }

    fn outputs(&self) -> Vec<super::Attachment> {
        self.inputs()
    }

    fn dependencies(&self) -> &Vec<super::NodeId> {
        &self.dependencies
    }

    fn dependencies_mut(&mut self) -> &mut Vec<super::NodeId> {
        &mut self.dependencies
    }
}

type OpaquePass = Pass;
type TransparentPass = Pass;

fn test(gpu: Rc<GpuInstance>) {
    let mut render_graph = RenderGraph::new(gpu);

    let depth_stencil = render_graph.create_texture(
        "depth-stencil",
        TextureInfo::black(wgpu::TextureFormat::Depth16Unorm),
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
        .with_pass(TransparentPass::new());

    render_graph.add_node(forward_pass);
}
