use super::{GraphNode, RenderPhase};
use crate::renderer::graph::{
    context::RenderContext,
    resources::{GraphResources, TextureId},
};

use rouge_ecs::{
    meta::{Access, AccessMeta, AccessType},
    ArgItem, SystemArg, World,
};

pub enum Attachment {
    Surface,
    Texture(TextureId),
}

pub struct ColorAttachment {
    pub attachment: Attachment,
    pub resolve_target: Option<Attachment>,
    pub store_op: wgpu::StoreOp,
    pub clear: bool,
}

pub struct DepthAttachment {
    pub attachment: Attachment,
    pub depth_store_op: wgpu::StoreOp,
    pub stencil_store_op: wgpu::StoreOp,
    pub clear_depth: Option<f32>,
    pub clear_stencil: Option<u32>,
}

pub struct RenderPass {
    phase: RenderPhase,
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
    subpasses: Vec<Subpass>,
}

impl RenderPass {
    pub fn new() -> Self {
        Self {
            phase: RenderPhase::Process,
            colors: Vec::new(),
            depth: None,
            subpasses: Vec::new(),
        }
    }

    pub fn set_phase(mut self, phase: RenderPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_color(
        mut self,
        attachment: Attachment,
        resolve_target: Option<Attachment>,
        store_op: wgpu::StoreOp,
        clear: bool,
    ) -> Self {
        self.colors.push(ColorAttachment {
            attachment,
            resolve_target,
            store_op,
            clear,
        });
        self
    }

    pub fn with_depth(
        mut self,
        attachment: Attachment,
        depth_store_op: wgpu::StoreOp,
        stencil_store_op: wgpu::StoreOp,
        clear_depth: Option<f32>,
        clear_stencil: Option<u32>,
    ) -> Self {
        self.depth = Some(DepthAttachment {
            attachment,
            depth_store_op,
            stencil_store_op,
            clear_depth,
            clear_stencil,
        });
        self
    }

    pub fn with_subpass(mut self, subpass: Subpass) -> Self {
        self.subpasses.push(subpass);
        self
    }

    pub fn add_subpass(&mut self, subpass: Subpass) -> &mut Self {
        self.subpasses.push(subpass);
        self
    }

    pub fn with_group<G: RenderGroup>(mut self, subpass: usize, group: G) -> Self {
        assert!(subpass < self.subpasses.len());
        self.subpasses[subpass].add_group(group);
        self
    }

    pub fn add_group<G: RenderGroup>(&mut self, subpass: usize, group: G) -> &mut Self {
        assert!(subpass < self.subpasses.len());
        self.subpasses[subpass].add_group(group);
        self
    }

    pub fn insert_group<G: RenderGroup>(
        &mut self,
        subpass: usize,
        index: usize,
        group: G,
    ) -> &mut Self {
        assert!(subpass < self.subpasses.len());
        self.subpasses[subpass].insert_group(index, group);
        self
    }

    fn begin<'a>(
        &self,
        ctx: &RenderContext<'a>,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &self
                .colors
                .iter()
                .map(|color| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: match color.attachment {
                            Attachment::Surface => {
                                ctx.resources().texture(GraphResources::SURFACE).view()
                            }
                            Attachment::Texture(ref id) => ctx.resources().texture(*id).view(),
                        },
                        ops: wgpu::Operations {
                            store: color.store_op,
                            load: match color.clear {
                                true => wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                false => wgpu::LoadOp::Load,
                            },
                        },
                        resolve_target: match color.resolve_target {
                            Some(ref attachment) => Some(match attachment {
                                Attachment::Surface => {
                                    ctx.resources().texture(GraphResources::SURFACE).view()
                                }
                                Attachment::Texture(ref id) => ctx.resources().texture(*id).view(),
                            }),
                            None => None,
                        },
                    })
                })
                .collect::<Vec<_>>(),
            depth_stencil_attachment: match self.depth {
                Some(ref depth) => Some(wgpu::RenderPassDepthStencilAttachment {
                    view: match depth.attachment {
                        Attachment::Surface => {
                            ctx.resources().texture(GraphResources::SURFACE).view()
                        }
                        Attachment::Texture(ref id) => ctx.resources().texture(*id).view(),
                    },
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth.clear_depth.unwrap_or(1.0)),
                        store: depth.depth_store_op,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth.clear_stencil.unwrap_or(0)),
                        store: depth.stencil_store_op,
                    }),
                }),
                None => None,
            },
            ..Default::default()
        })
    }
}

impl GraphNode for RenderPass {
    fn execute(&self, ctx: RenderContext) {
        let mut encoder = ctx.create_encoder();
        {
            let mut render_pass = self.begin(&ctx, &mut encoder);

            for subpass in &self.subpasses {
                subpass.execute(ctx.system_arg::<&World>(), &mut render_pass);
            }
        }

        ctx.submit(encoder);
    }

    fn phase(&self) -> RenderPhase {
        self.phase
    }

    fn access(&self) -> Vec<rouge_ecs::meta::AccessMeta> {
        let mut writes = Vec::new();

        for color in &self.colors {
            match color.attachment {
                Attachment::Surface => writes.push(AccessMeta::new(
                    AccessType::Id(GraphResources::SURFACE.into()),
                    Access::Write,
                )),
                Attachment::Texture(id) => {
                    writes.push(AccessMeta::new(AccessType::Id(id), Access::Write))
                }
            }

            if let Some(ref resolve_target) = color.resolve_target {
                match resolve_target {
                    Attachment::Surface => writes.push(AccessMeta::new(
                        AccessType::Id(GraphResources::SURFACE.into()),
                        Access::Write,
                    )),
                    Attachment::Texture(id) => {
                        writes.push(AccessMeta::new(AccessType::Id(*id), Access::Write))
                    }
                }
            }
        }

        if let Some(depth) = &self.depth {
            match depth.attachment {
                Attachment::Surface => writes.push(AccessMeta::new(
                    AccessType::Id(GraphResources::SURFACE.into()),
                    Access::Write,
                )),
                Attachment::Texture(id) => {
                    writes.push(AccessMeta::new(AccessType::Id(id), Access::Write))
                }
            }
        }

        writes
    }
}

pub trait RenderGroup: Send + Sync + 'static {
    type Arg: SystemArg;

    fn render<'a>(&self, arg: ArgItem<'a, Self::Arg>, pass: &mut wgpu::RenderPass);
}

pub struct RenderGroupInstance {
    execute: Box<dyn Fn(&mut wgpu::RenderPass, &World) + Send + Sync>,
}

impl RenderGroupInstance {
    pub fn new<'a, G: RenderGroup>(group: G) -> Self {
        Self {
            execute: Box::new(move |pass, world| {
                let arg = G::Arg::get(world);
                group.render(arg, pass);
            }),
        }
    }

    pub fn execute(&self, world: &World, pass: &mut wgpu::RenderPass) {
        (self.execute)(pass, world);
    }
}

pub struct Subpass {
    groups: Vec<RenderGroupInstance>,
}

impl Subpass {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn with_group<G: RenderGroup>(mut self, group: G) -> Self {
        let group = RenderGroupInstance::new(group);
        self.groups.push(group);
        self
    }

    pub fn add_group<G: RenderGroup>(&mut self, group: G) -> &mut Self {
        let group = RenderGroupInstance::new(group);
        self.groups.push(group);
        self
    }

    pub fn insert_group<G: RenderGroup>(&mut self, index: usize, group: G) -> &mut Self {
        let group = RenderGroupInstance::new(group);
        self.groups.insert(index, group);
        self
    }

    pub(super) fn execute(&self, world: &World, pass: &mut wgpu::RenderPass) {
        for group in &self.groups {
            group.execute(world, pass);
        }
    }
}
