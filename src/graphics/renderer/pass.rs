use std::{
    num::{NonZeroU32, NonZeroU64},
    rc::Rc,
};

use super::{context::RenderContext, NodeId, RenderGraph, RenderGraphNode};
use crate::graphics::{
    core::gpu::GpuInstance,
    resources::{
        texture::{TextureInfo, ToTextureViewDimension},
        BufferId, TextureId,
    },
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

    fn dependencies(&self) -> &Vec<super::NodeId> {
        &self.dependencies
    }

    fn dependencies_mut(&mut self) -> &mut Vec<super::NodeId> {
        &mut self.dependencies
    }
}

pub enum AttachmentBindingType {
    Texture(TextureId),
    Buffer {
        buffer: BufferId,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    },
}

pub struct AttachmentBinding {
    ty: AttachmentBindingType,
    visibility: wgpu::ShaderStages,
    count: Option<NonZeroU32>,
}

pub struct ShaderBindGroup {
    bindings: Vec<AttachmentBinding>,
}

impl ShaderBindGroup {
    pub fn new() -> ShaderBindGroup {
        ShaderBindGroup {
            bindings: Vec::new(),
        }
    }

    pub fn with_texture_binding(
        mut self,
        id: impl Into<BufferId>,
        visibility: wgpu::ShaderStages,
        count: Option<NonZeroU32>,
    ) -> ShaderBindGroup {
        self.bindings.push(AttachmentBinding {
            ty: AttachmentBindingType::Texture(id.into()),
            visibility,
            count,
        });
        self
    }

    pub fn with_buffer_binding(
        mut self,
        id: impl Into<BufferId>,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
        visibility: wgpu::ShaderStages,
        count: Option<NonZeroU32>,
    ) -> ShaderBindGroup {
        self.bindings.push(AttachmentBinding {
            ty: AttachmentBindingType::Buffer {
                buffer: id.into(),
                ty,
                has_dynamic_offset,
                min_binding_size,
            },
            visibility,
            count,
        });
        self
    }

    fn bind_group_layout_entries(&self, graph: &RenderGraph) -> Vec<wgpu::BindGroupLayoutEntry> {
        self.bindings
            .iter()
            .enumerate()
            .map(|(idx, binding)| wgpu::BindGroupLayoutEntry {
                binding: idx as u32,
                visibility: binding.visibility,
                ty: match &binding.ty {
                    AttachmentBindingType::Texture(id) => {
                        let texture = graph.dyn_texture(&id).expect("Texture not found.");

                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: texture.dimension().to_texture_view_dimension(),
                            multisampled: false,
                        }
                    }
                    AttachmentBindingType::Buffer {
                        ty,
                        has_dynamic_offset,
                        min_binding_size,
                        ..
                    } => wgpu::BindingType::Buffer {
                        ty: *ty,
                        has_dynamic_offset: *has_dynamic_offset,
                        min_binding_size: *min_binding_size,
                    },
                },
                count: binding.count,
            })
            .collect::<Vec<_>>()
    }
}

type ComputePassExecutor = dyn Fn(&RenderContext, &[wgpu::BindGroup], &mut wgpu::ComputePass);

pub struct ComputeSubpass {
    bind_group_layouts: Vec<ShaderBindGroup>,
    bind_groups: Vec<wgpu::BindGroup>,
    pipeline: Option<wgpu::ComputePipeline>,
    executor: Option<Box<ComputePassExecutor>>,
}

impl ComputeSubpass {
    pub fn new() -> ComputeSubpass {
        ComputeSubpass {
            bind_group_layouts: Vec::new(),
            bind_groups: Vec::new(),
            pipeline: None,
            executor: None,
        }
    }

    pub fn with_bind_group(mut self, bind_group: ShaderBindGroup) -> ComputeSubpass {
        self.bind_group_layouts.push(bind_group);
        self
    }

    pub fn with_bind_groups(mut self, bind_groups: Vec<ShaderBindGroup>) -> ComputeSubpass {
        self.bind_group_layouts = bind_groups;
        self
    }

    pub fn with_executor<
        T: Fn(&RenderContext, &[wgpu::BindGroup], &mut wgpu::ComputePass) + 'static,
    >(
        mut self,
        executor: T,
    ) -> ComputeSubpass {
        self.executor = Some(Box::new(executor));
        self
    }

    pub fn build(
        mut self,
        graph: &RenderGraph,
        shader: &wgpu::ShaderModule,
        entry: &str,
    ) -> ComputeSubpass {
        let layout_entries = self
            .bind_group_layouts
            .iter()
            .map(|bg| bg.bind_group_layout_entries(graph))
            .collect::<Vec<_>>();

        let bind_groups =
            layout_entries
                .iter()
                .map(|l| {
                    let layout = graph.gpu.device().create_bind_group_layout(
                        &wgpu::BindGroupLayoutDescriptor {
                            entries: l.as_slice(),
                            label: None,
                        },
                    );

                    let bind_group =
                        graph
                            .gpu()
                            .device()
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                label: None,
                                layout: &layout,
                                entries: &[],
                            });

                    (layout, bind_group)
                })
                .collect::<Vec<_>>();

        let pipeline_layout = if bind_groups.len() > 0 {
            Some(
                graph
                    .gpu
                    .device()
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: bind_groups
                            .iter()
                            .map(|(l, _)| l)
                            .collect::<Vec<_>>()
                            .as_slice(),
                        push_constant_ranges: &[],
                    }),
            )
        } else {
            None
        };

        let pipeline =
            graph
                .gpu
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    entry_point: entry,
                    layout: pipeline_layout.as_ref(),
                    module: shader,
                });

        self.pipeline = Some(pipeline);

        let groups: (Vec<wgpu::BindGroupLayout>, Vec<wgpu::BindGroup>) =
            bind_groups.into_iter().unzip();

        self.bind_groups = groups.1;

        self
    }

    pub fn execute<'a>(&'a self, ctx: &RenderContext, pass: &mut wgpu::ComputePass<'a>) {
        if let Some(pipeline) = &self.pipeline {
            pass.set_pipeline(pipeline);
        }

        let executer = self
            .executor
            .as_ref()
            .expect("No executor set for compute pass.");

        executer(ctx, &self.bind_groups, pass);
    }
}

pub struct ComputePassNode {
    id: NodeId,
    dependencies: Vec<NodeId>,
    subpasses: Vec<ComputeSubpass>,
}

impl ComputePassNode {
    pub fn new(id: impl Into<NodeId>) -> ComputePassNode {
        ComputePassNode {
            id: id.into(),
            dependencies: Vec::new(),
            subpasses: Vec::new(),
        }
    }

    pub fn with_subpass(mut self, subpass: ComputeSubpass) -> ComputePassNode {
        self.subpasses.push(subpass);
        self
    }
}

impl RenderGraphNode for ComputePassNode {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn dependencies(&self) -> &Vec<NodeId> {
        &self.dependencies
    }

    fn dependencies_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.dependencies
    }

    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        for subpass in &self.subpasses {
            subpass.execute(ctx, &mut pass);
        }
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
            .with_executor(|ctx, bind_groups, pass| {}),
    );

    render_graph.add_node(forward_pass);
}
