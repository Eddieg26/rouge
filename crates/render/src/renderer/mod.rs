use crate::types::Color;
use crate::{device::RenderDevice, resources::Id};
use std::collections::HashMap;
use std::hash::Hash;
use wgpu::TextureDescriptor;

pub mod context;
pub mod graph;
pub mod state;

pub struct TextureDesc {
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

pub struct BufferDesc {
    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
}

pub enum RenderGraphResource {
    Texture {
        id: Id<RenderGraphTexture>,
        desc: TextureDesc,
    },
    Buffer {
        id: Id<RenderGraphBuffer>,
        desc: BufferDesc,
    },
}

pub struct RenderGraphTexture {
    pub view: wgpu::TextureView,
    pub desc: Option<TextureDesc>,
}

impl std::ops::Deref for RenderGraphTexture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl AsRef<wgpu::TextureView> for RenderGraphTexture {
    fn as_ref(&self) -> &wgpu::TextureView {
        &self.view
    }
}

pub struct RenderGraphBuffer {
    pub buffer: wgpu::Buffer,
}

impl std::ops::Deref for RenderGraphBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl AsRef<wgpu::Buffer> for RenderGraphBuffer {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

pub struct RenderGraphResources {
    textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
    buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
}

impl RenderGraphResources {
    pub fn new(
        textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
        buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
    ) -> Self {
        Self { textures, buffers }
    }

    pub fn texture(&self, id: &Id<RenderGraphTexture>) -> Option<&RenderGraphTexture> {
        self.textures.get(id)
    }

    pub fn buffer(&self, id: &Id<RenderGraphBuffer>) -> Option<&RenderGraphBuffer> {
        self.buffers.get(id)
    }

    pub fn import_texture(
        &mut self,
        id: impl Into<Id<RenderGraphTexture>>,
        texture: wgpu::TextureView,
        desc: Option<TextureDesc>,
    ) -> Id<RenderGraphTexture> {
        let id = id.into();
        self.textures.insert(
            id,
            RenderGraphTexture {
                view: texture,
                desc,
            },
        );

        id
    }

    pub fn import_buffer(
        &mut self,
        id: impl Into<Id<RenderGraphBuffer>>,
        buffer: wgpu::Buffer,
    ) -> Id<RenderGraphBuffer> {
        let id = id.into();
        self.buffers
            .insert(id, RenderGraphBuffer { buffer});

        id
    }

    pub fn remove_texture(&mut self, id: &Id<RenderGraphTexture>) {
        self.textures.remove(id);
    }

    pub fn remove_buffer(&mut self, id: &Id<RenderGraphBuffer>) {
        self.buffers.remove(id);
    }

    pub fn resize(&mut self, device: &RenderDevice, width: u32, height: u32) {
        for texture in &mut self.textures.values_mut() {
            texture.view = {
                let Some(desc) = texture.desc.as_ref() else {
                    continue;
                };

                let texture = device.create_texture(&TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 0,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format,
                    usage: desc.usage,
                    view_formats: &[desc.format],
                });

                texture.create_view(&Default::default())
            };
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attachment {
    Surface,
    Texture(Id<RenderGraphTexture>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StoreOp {
    Store,
    Clear,
}

impl Into<wgpu::StoreOp> for StoreOp {
    fn into(self) -> wgpu::StoreOp {
        match self {
            StoreOp::Store => wgpu::StoreOp::Store,
            StoreOp::Clear => wgpu::StoreOp::Discard,
        }
    }
}

pub enum LoadOp<T> {
    Clear(T),
    Load,
}

impl<T> Into<wgpu::LoadOp<T>> for LoadOp<T> {
    fn into(self) -> wgpu::LoadOp<T> {
        match self {
            LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

pub struct Operations<T> {
    pub load: LoadOp<T>,
    pub store: StoreOp,
}

impl<T> Into<wgpu::Operations<T>> for Operations<T> {
    fn into(self) -> wgpu::Operations<T> {
        wgpu::Operations {
            load: self.load.into(),
            store: self.store.into(),
        }
    }
}

pub struct ColorAttachment {
    pub attachment: Attachment,
    pub resolve_target: Option<Attachment>,
    pub store_op: StoreOp,
    pub clear: Option<Color>,
}

pub struct DepthAttachment {
    pub attachment: Attachment,
    pub depth_store_op: Operations<f32>,
    pub stencil_store_op: Option<Operations<u32>>,
}

pub struct RenderPass {
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
}

impl RenderPass {
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            depth: None,
        }
    }

    pub fn with_color(
        mut self,
        attachment: Attachment,
        resolve_target: Option<Attachment>,
        store_op: StoreOp,
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
        depth_store_op: Operations<f32>,
        stencil_store_op: Option<Operations<u32>>,
    ) -> Self {
        self.depth = Some(DepthAttachment {
            attachment,
            depth_store_op,
            stencil_store_op,
        });

        self
    }

    pub fn begin<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        resources: &'a RenderGraphResources,
        target: &'a wgpu::TextureView,
        depth: &'a wgpu::TextureView,
        clear: Option<Color>,
    ) -> Option<wgpu::RenderPass<'a>> {
        let mut color_attachments = vec![];
        for color in self.colors.iter() {
            let view = match color.attachment {
                Attachment::Surface => target,
                Attachment::Texture(ref id) => &resources.texture(id)?.view,
            };

            let resolve_target = match color.resolve_target {
                Some(attachment) => match attachment {
                    Attachment::Surface => Some(target),
                    Attachment::Texture(ref id) => Some(resources.texture(id)?).map(|v| &v.view),
                },
                None => None,
            };

            let load = match clear {
                Some(color) => wgpu::LoadOp::Clear(color.into()),
                None => match color.clear {
                    Some(color) => wgpu::LoadOp::Clear(color.into()),
                    None => wgpu::LoadOp::Load,
                },
            };

            let attachement = wgpu::RenderPassColorAttachment {
                view,
                resolve_target,
                ops: wgpu::Operations {
                    load,
                    store: color.store_op.into(),
                },
            };

            color_attachments.push(Some(attachement));
        }

        let depth_stencil_attachment = match &self.depth {
            Some(attachment) => Some(wgpu::RenderPassDepthStencilAttachment {
                view: match attachment.attachment {
                    Attachment::Surface => depth,
                    Attachment::Texture(ref id) => resources.texture(id)?,
                },
                depth_ops: Some(wgpu::Operations {
                    load: match attachment.depth_store_op.load {
                        LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                        LoadOp::Load => wgpu::LoadOp::Load,
                    },
                    store: attachment.depth_store_op.store.into(),
                }),
                stencil_ops: attachment
                    .stencil_store_op
                    .as_ref()
                    .map(|op| wgpu::Operations {
                        load: match op.load {
                            LoadOp::Clear(value) => wgpu::LoadOp::Clear(value),
                            LoadOp::Load => wgpu::LoadOp::Load,
                        },
                        store: op.store.into(),
                    }),
            }),
            None => None,
        };

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            ..Default::default()
        });

        Some(pass)
    }
}
