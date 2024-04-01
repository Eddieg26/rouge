use super::{GraphNode, RenderPhase};
use crate::{
    core::{
        device::RenderDevice,
        draw::{Draw, DrawCalls, Render, Renders},
        surface::RenderSurface,
        ty::color::Color,
    },
    renderer::graph::{
        context::RenderContext,
        resources::{GraphResources, TextureId},
        RenderGraph,
    },
    resources::texture::{DepthTextures, TextureInfo},
};
use rouge_ecs::{
    meta::{Access, AccessMeta, AccessType},
    ArgItem, ResourceId, SystemArg, World,
};
use std::{any::TypeId, collections::HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attachment {
    Surface,
    Texture(TextureId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderInterval {
    PerRender,
    PerFrame,
}

pub struct ColorAttachment {
    pub attachment: Attachment,
    pub resolve_target: Option<Attachment>,
    pub store_op: wgpu::StoreOp,
    pub clear: Option<Color>,
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
    interval: RenderInterval,
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
    subpasses: Vec<Subpass>,
}

impl RenderPass {
    pub fn new() -> Self {
        Self {
            phase: RenderPhase::Process,
            interval: RenderInterval::PerFrame,
            colors: Vec::new(),
            depth: None,
            subpasses: Vec::new(),
        }
    }

    pub fn set_phase(mut self, phase: RenderPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn set_interval(mut self, interval: RenderInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn with_color(
        mut self,
        attachment: Attachment,
        resolve_target: Option<Attachment>,
        store_op: wgpu::StoreOp,
        clear: Option<Color>,
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

    pub fn with_group<D: Draw, G: RenderGroup<D>>(mut self, subpass: usize, group: G) -> Self {
        assert!(subpass < self.subpasses.len());
        self.subpasses[subpass].add_group(group);
        self
    }

    pub fn add_group<D: Draw, G: RenderGroup<D>>(&mut self, subpass: usize, group: G) -> &mut Self {
        assert!(subpass < self.subpasses.len());
        self.subpasses[subpass].add_group(group);
        self
    }

    fn begin<'a>(
        &self,
        ctx: &RenderContext<'a>,
        encoder: &'a mut wgpu::CommandEncoder,
        render: &dyn Render,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &self
                .colors
                .iter()
                .map(|color| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: match color.attachment {
                            Attachment::Surface => ctx
                                .resources()
                                .texture(ResourceId::from(GraphResources::SURFACE_ID))
                                .expect("Surface texture not found"),

                            Attachment::Texture(id) => ctx.resources().texture_unchecked(id),
                        },
                        ops: wgpu::Operations {
                            store: color.store_op,
                            load: render
                                .clear()
                                .and_then(|c| Some(wgpu::LoadOp::Clear(c.into())))
                                .unwrap_or(wgpu::LoadOp::Load),
                        },
                        resolve_target: match color.resolve_target {
                            Some(ref attachment) => Some(match attachment {
                                Attachment::Surface => ctx
                                    .resources()
                                    .texture(ResourceId::from(GraphResources::SURFACE_ID))
                                    .expect("Surface texture not found"),
                                Attachment::Texture(id) => ctx.resources().texture_unchecked(*id),
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
                            let depth_texture_id = DepthTextures::texture_id(render.depth());
                            ctx.resources()
                                .texture(depth_texture_id)
                                .expect("Depth texture not found")
                        }
                        Attachment::Texture(ref id) => ctx.resources().texture_unchecked(*id),
                    },
                    depth_ops: Some(wgpu::Operations {
                        load: match depth.clear_depth {
                            Some(clear) => wgpu::LoadOp::Clear(clear),
                            None => wgpu::LoadOp::Load,
                        },
                        store: depth.depth_store_op,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: match depth.clear_stencil {
                            Some(clear) => wgpu::LoadOp::Clear(clear),
                            None => wgpu::LoadOp::Load,
                        },
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
            let renders = ctx.system_arg::<&Renders>();
            for render in renders.iter() {
                let mut render_pass = self.begin(&ctx, &mut encoder, render);
                for subpass in &self.subpasses {
                    subpass.execute(ctx.system_arg::<&World>(), &mut render_pass, render);
                }
            }
        }

        ctx.submit(encoder);
    }

    fn phase(&self) -> RenderPhase {
        self.phase
    }

    fn access(&self) -> Vec<rouge_ecs::meta::AccessMeta> {
        let mut access = Vec::new();

        let surface_id = ResourceId::from(GraphResources::SURFACE_ID);
        let mut ids_added = HashSet::new();

        for color in &self.colors {
            match &color.attachment {
                Attachment::Surface => {
                    if !ids_added.contains(&surface_id) {
                        access.push(AccessMeta::new(AccessType::id(surface_id), Access::Write));
                        ids_added.insert(surface_id);
                    }
                }
                Attachment::Texture(id) => {
                    if !ids_added.contains(id) {
                        access.push(AccessMeta::new(AccessType::Id(*id), Access::Write));
                        ids_added.insert(*id);
                    }
                }
            }

            if let Some(resolve_target) = &color.resolve_target {
                match resolve_target {
                    Attachment::Surface => {
                        if !ids_added.contains(&surface_id) {
                            access.push(AccessMeta::new(AccessType::id(surface_id), Access::Write));
                            ids_added.insert(surface_id);
                        }
                    }
                    Attachment::Texture(id) => {
                        if !ids_added.contains(id) {
                            access.push(AccessMeta::new(AccessType::Id(*id), Access::Write));
                            ids_added.insert(*id);
                        }
                    }
                }
            }
        }

        if let Some(depth) = &self.depth {
            match &depth.attachment {
                Attachment::Surface => access.push(AccessMeta::new(
                    AccessType::resource::<DepthTextures>(),
                    Access::Read,
                )),
                Attachment::Texture(id) => {
                    if !ids_added.contains(id) {
                        access.push(AccessMeta::new(AccessType::Id(*id), Access::Write));
                        ids_added.insert(*id);
                    }
                }
            }
        }

        for subpass in &self.subpasses {
            access.append(&mut subpass.access());
        }

        access.push(AccessMeta::new(
            AccessType::resource::<Renders>(),
            Access::Read,
        ));

        access
    }
}

pub trait RenderGroup<D: Draw>: Send + Sync + 'static {
    type Arg: SystemArg;

    fn render<'a>(
        &self,
        pass: &mut wgpu::RenderPass,
        arg: ArgItem<'a, Self::Arg>,
        render: &D::Render,
        calls: &DrawCalls<D>,
    );
}

pub struct RenderGroupInstance {
    execute: Box<dyn Fn(&mut wgpu::RenderPass, &World, &dyn Render) + Send + Sync>,
    access: fn() -> Vec<AccessMeta>,
    render_type: TypeId,
    priority: u16,
}

impl RenderGroupInstance {
    pub fn new<'a, D: Draw, G: RenderGroup<D>>(group: G) -> Self {
        Self {
            render_type: TypeId::of::<D::Render>(),
            priority: D::PRIORITY,
            access: || {
                let mut access = G::Arg::metas();
                access.push(AccessMeta::new(
                    AccessType::resource::<DrawCalls<D>>(),
                    Access::Read,
                ));
                access
            },
            execute: Box::new(move |pass, world, render| {
                let arg = G::Arg::get(world);
                let render = render.as_any().downcast_ref::<D::Render>().unwrap();
                let calls = world.resource::<DrawCalls<D>>();
                group.render(pass, arg, render, calls);
            }),
        }
    }

    pub fn execute(&self, world: &World, pass: &mut wgpu::RenderPass, render: &dyn Render) {
        (self.execute)(pass, world, render);
    }

    pub fn priority(&self) -> u16 {
        self.priority
    }

    fn access(&self) -> Vec<AccessMeta> {
        (self.access)()
    }
}

pub struct Subpass {
    groups: Vec<RenderGroupInstance>,
}

impl Subpass {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn with_group<D: Draw, G: RenderGroup<D>>(mut self, group: G) -> Self {
        let group = RenderGroupInstance::new(group);
        self.groups.push(group);
        self.groups.sort_by(|a, b| a.priority().cmp(&b.priority()));
        self
    }

    pub fn add_group<D: Draw, G: RenderGroup<D>>(&mut self, group: G) -> &mut Self {
        let group = RenderGroupInstance::new(group);
        self.groups.push(group);
        self.groups.sort_by(|a, b| a.priority().cmp(&b.priority()));
        self
    }

    pub(super) fn execute(&self, world: &World, pass: &mut wgpu::RenderPass, render: &dyn Render) {
        for group in &self.groups {
            if render.as_any().type_id() == group.render_type {
                group.execute(world, pass, render);
            }
        }
    }

    fn access(&self) -> Vec<AccessMeta> {
        let mut access = Vec::new();

        for group in &self.groups {
            access.append(&mut group.access());
        }

        access
    }
}

fn create_depth_textures(
    device: &RenderDevice,
    surface: &RenderSurface,
    renders: &Renders,
    graph: &mut RenderGraph,
    depths: &mut DepthTextures,
) {
    let size = surface.size();

    depths.reset();
    for render in renders.iter() {
        let depth = render.depth();
        let texture_id = DepthTextures::texture_id(depth);
        if graph.texture(texture_id).is_none() {
            let texture = device.create_texture(
                &TextureInfo::new(size.width, size.height)
                    .format(surface.depth_format())
                    .descriptor(),
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            graph.import_texture(texture_id, view);
        }

        depths.insert(depth);
    }

    for removed in depths.retain() {
        let removed_id = DepthTextures::texture_id(removed);
        graph.remove_texture(removed_id);
    }
}
