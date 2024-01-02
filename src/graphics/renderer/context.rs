use crate::{
    ecs::{resource::ResourceId, World},
    graphics::{
        core::device::RenderDevice,
        resources::{
            buffer::Buffer,
            texture::{Texture, Texture2d, TextureDesc},
            BufferId, TextureId,
        },
        state::RenderState,
    },
};
use std::collections::HashMap;

pub struct RenderContext<'a> {
    world: &'a World,
    device: &'a RenderDevice,
    state: &'a RenderState,
    resources: &'a GraphResources,
    render_target: &'a wgpu::TextureView,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        world: &'a World,
        device: &'a RenderDevice,
        state: &'a RenderState,
        resources: &'a GraphResources,
        render_target: &'a wgpu::TextureView,
    ) -> RenderContext<'a> {
        RenderContext {
            device,
            state,
            world,
            resources,
            render_target,
        }
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn state(&self) -> &RenderState {
        self.state
    }

    pub fn resources(&self) -> &GraphResources {
        self.resources
    }

    pub fn world(&self) -> &World {
        self.world
    }

    pub fn render_target(&self) -> &wgpu::TextureView {
        self.render_target
    }
}

pub struct RenderUpdateContext<'a> {
    device: &'a RenderDevice,
    resources: &'a GraphResources,
}

impl<'a> RenderUpdateContext<'a> {
    pub fn new(device: &'a RenderDevice, resources: &'a GraphResources) -> RenderUpdateContext<'a> {
        RenderUpdateContext { device, resources }
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn resources(&self) -> &GraphResources {
        self.resources
    }
}

pub enum GraphResource {
    Texture {
        usages: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
        dimension: wgpu::TextureDimension,
    },
    Buffer {
        usage: wgpu::BufferUsages,
        size: wgpu::BufferAddress,
    },
}

pub struct GraphResources {
    infos: HashMap<ResourceId, GraphResource>,
    textures: HashMap<TextureId, Box<dyn Texture>>,
    buffers: HashMap<BufferId, Buffer>,
}

impl GraphResources {
    pub fn new() -> GraphResources {
        GraphResources {
            infos: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn create_texture(
        &mut self,
        id: impl Into<TextureId>,
        usages: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
        dimension: wgpu::TextureDimension,
    ) -> TextureId {
        let id = id.into();
        self.infos.insert(
            id,
            GraphResource::Texture {
                usages,
                format,
                dimension,
            },
        );

        id
    }

    pub fn create_buffer(
        &mut self,
        id: impl Into<TextureId>,
        usages: wgpu::BufferUsages,
        size: wgpu::BufferAddress,
    ) -> BufferId {
        let id = id.into();
        self.infos.insert(
            id,
            GraphResource::Buffer {
                usage: usages,
                size,
            },
        );

        id
    }

    pub fn import_texture<T: Texture>(
        &mut self,
        id: impl Into<TextureId>,
        texture: T,
    ) -> TextureId {
        let id = id.into();
        self.textures.insert(id, Box::new(texture));

        id
    }

    pub fn import_buffer(&mut self, id: impl Into<BufferId>, buffer: Buffer) -> BufferId {
        let id = id.into();
        self.buffers.insert(id, buffer);

        id
    }

    pub fn texture<T: Texture>(&self, id: &TextureId) -> Option<&T> {
        self.textures
            .get(id)
            .map(|t| t.as_any().downcast_ref::<T>().unwrap())
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.textures.get(id).map(|t| t.as_ref())
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    pub(super) fn update(&mut self, device: &RenderDevice, width: u32, height: u32) {
        for (id, info) in &self.infos {
            match info {
                GraphResource::Texture {
                    usages,
                    format,
                    dimension,
                } => {
                    let texture: Box<dyn Texture> = match dimension {
                        wgpu::TextureDimension::D1 => todo!(),
                        wgpu::TextureDimension::D2 => Box::new(Texture2d::from_desc(
                            device.inner(),
                            &TextureDesc {
                                depth: 1,
                                width,
                                height,
                                format: *format,
                                dimension: *dimension,
                            },
                        )),
                        wgpu::TextureDimension::D3 => todo!(),
                    };

                    self.textures.insert(*id, texture);
                }
                GraphResource::Buffer { usage, size } => {
                    let buffer = Buffer::from_size(device.inner(), *usage, *size);

                    self.buffers.insert(*id, buffer);
                }
            }
        }
    }
}
