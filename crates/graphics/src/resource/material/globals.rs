use super::MaterialBinding;
use crate::{
    resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutBuilder, UniformBuffer, UniformBufferArray,
    },
    RenderDevice, RenderResourceExtractor, RenderViewData, View,
};
use ecs::{core::resource::Resource, system::unlifetime::ReadRes};
use encase::ShaderType;
use wgpu::{BufferBindingType, ShaderStages};

#[derive(Debug, Default, Copy, Clone, ShaderType)]
pub struct FrameData {
    pub frame: u32,
    pub time: f32,
    pub delta_time: f32,
    _padding: f32,
}

pub struct GlobalView<V: View> {
    binding: BindGroup,
    layout: BindGroupLayout,
    buffer: UniformBuffer<FrameData>,
    views: UniformBufferArray<RenderViewData>,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: View> GlobalView<V> {
    pub fn new(device: &RenderDevice) -> Self {
        let buffer = UniformBuffer::new(device, FrameData::default());
        let mut views = UniformBufferArray::<RenderViewData>::aligned(device);
        views.push(&RenderViewData::default());
        views.update(device);

        let entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: views.binding().unwrap(),
            },
        ];

        let layout = BindGroupLayoutBuilder::new()
            .with_buffer(
                0,
                ShaderStages::all(),
                BufferBindingType::Uniform,
                Some(FrameData::min_size()),
                None,
            )
            .with_buffer(
                1,
                ShaderStages::all(),
                BufferBindingType::Uniform,
                Some(RenderViewData::min_size()),
                None,
            )
            .build(device);

        Self {
            binding: BindGroup::create(device, &layout, &entries, ()),
            layout,
            buffer,
            views,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    pub fn buffer(&self) -> &UniformBuffer<FrameData> {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut UniformBuffer<FrameData> {
        &mut self.buffer
    }

    pub fn views(&self) -> &UniformBufferArray<RenderViewData> {
        &self.views
    }

    pub fn views_mut(&mut self) -> &mut UniformBufferArray<RenderViewData> {
        &mut self.views
    }
}

impl<V: View> Resource for GlobalView<V> {}

impl<V: View> MaterialBinding for GlobalView<V> {
    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

impl<V: View> RenderResourceExtractor for GlobalView<V> {
    type Arg = ReadRes<RenderDevice>;

    fn can_extract(world: &ecs::world::World) -> bool {
        world.has_resource::<RenderDevice>()
    }

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self, crate::ExtractError> {
        Ok(Self::new(&arg))
    }
}
