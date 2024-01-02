use crate::graphics::{
    renderer::{
        context::{RenderContext, RenderUpdateContext},
        graph::RenderGraphNode,
        NodeId,
    },
    resources::{texture::ToTextureViewDimension, BufferId, TextureId},
};
use std::{
    num::{NonZeroU32, NonZeroU64},
    rc::Rc,
};

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

    fn bind_group_layout_entries(
        &self,
        ctx: &RenderUpdateContext,
    ) -> Vec<wgpu::BindGroupLayoutEntry> {
        self.bindings
            .iter()
            .enumerate()
            .map(|(idx, binding)| wgpu::BindGroupLayoutEntry {
                binding: idx as u32,
                visibility: binding.visibility,
                ty: match &binding.ty {
                    AttachmentBindingType::Texture(id) => {
                        let texture = ctx
                            .resources()
                            .dyn_texture(&id)
                            .expect("Texture not found.");

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
    shader: Option<Rc<wgpu::ShaderModule>>,
    entry: String,
    pipeline: Option<wgpu::ComputePipeline>,
    executor: Option<Box<ComputePassExecutor>>,
}

impl ComputeSubpass {
    pub fn new() -> ComputeSubpass {
        ComputeSubpass {
            bind_group_layouts: Vec::new(),
            bind_groups: Vec::new(),
            shader: None,
            entry: String::new(),
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

    pub fn with_shader(mut self, shader: Rc<wgpu::ShaderModule>, entry: &str) -> ComputeSubpass {
        self.shader = Some(shader);
        self.entry = entry.to_string();
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

    pub fn build(&mut self, ctx: &RenderUpdateContext) {
        let shader = self
            .shader
            .as_ref()
            .expect("No shader set for compute pass.");
        let layout_entries = self
            .bind_group_layouts
            .iter()
            .map(|bg| bg.bind_group_layout_entries(ctx))
            .collect::<Vec<_>>();

        let bind_groups = layout_entries
            .iter()
            .map(|l| {
                let layout = ctx.device().inner().create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        entries: l.as_slice(),
                        label: None,
                    },
                );

                let bind_group =
                    ctx.device()
                        .inner()
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
                ctx.device()
                    .inner()
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
            ctx.device()
                .inner()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    entry_point: &self.entry,
                    layout: pipeline_layout.as_ref(),
                    module: &shader,
                });

        self.pipeline = Some(pipeline);

        let groups: (Vec<wgpu::BindGroupLayout>, Vec<wgpu::BindGroup>) =
            bind_groups.into_iter().unzip();

        self.bind_groups = groups.1;
    }

    fn update(&mut self, ctx: &RenderUpdateContext) {
        if self.pipeline.is_none() {
            self.build(ctx);
        }
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

    fn update(&mut self, ctx: &RenderUpdateContext) {
        for subpass in &mut self.subpasses {
            subpass.update(ctx);
        }
    }
}
