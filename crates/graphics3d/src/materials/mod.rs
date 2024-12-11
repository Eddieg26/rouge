use asset::io::cache::LoadPath;
use ecs::{core::resource::Resource, system::unlifetime::ReadRes};
use graphics::{
    encase::ShaderType,
    resource::{
        globals::GlobalView, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutBuilder,
        MaterialBinding, MeshPipeline, UniformBuffer, VertexAttribute,
    },
    wgpu::{PrimitiveState, ShaderStages},
    RenderDevice, RenderResourceExtractor,
};

pub mod standard;
pub mod unlit;

pub struct Mesh3d {
    layout: BindGroupLayout,
    binding: BindGroup,
    buffer: UniformBuffer<glam::Mat4>,
}

impl Mesh3d {
    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn buffer(&self) -> &UniformBuffer<glam::Mat4> {
        &self.buffer
    }

    pub fn update(&mut self, device: &RenderDevice, value: glam::Mat4) {
        self.buffer.set(value);
        self.buffer.update(device);
    }
}

impl MeshPipeline for Mesh3d {
    type View = GlobalView<()>;
    type Mesh = Mesh3d;

    fn primitive() -> PrimitiveState {
        PrimitiveState::default()
    }

    fn attributes() -> Vec<VertexAttribute> {
        vec![VertexAttribute::Vec3, VertexAttribute::Vec2]
    }

    fn shader() -> impl Into<LoadPath> {
        "graphics3d://assets/mesh3d.wgsl"
    }
}

impl MaterialBinding for Mesh3d {
    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

impl RenderResourceExtractor for Mesh3d {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(device: ecs::system::ArgItem<Self::Arg>) -> Result<Self, graphics::ExtractError> {
        let layout = BindGroupLayoutBuilder::new()
            .with_uniform_buffer(
                0,
                ShaderStages::VERTEX,
                true,
                Some(glam::Mat4::min_size()),
                None,
            )
            .build(&device);

        let buffer = UniformBuffer::new(&device, glam::Mat4::IDENTITY);

        let entries = BindGroupEntries::new().with_buffer(0, &buffer, 0, None);
        let binding = BindGroup::create(&device, &layout, &entries, ());

        Ok(Self {
            layout,
            binding,
            buffer,
        })
    }
}

impl Resource for Mesh3d {}
