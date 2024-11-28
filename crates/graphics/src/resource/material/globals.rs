use crate::{
    resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutBuilder, UniformBuffer, UniformBufferArray,
    },
    RenderDevice, RenderResourceExtractor, RenderViewData,
};
use ecs::{
    core::resource::Resource,
    system::{unlifetime::ReadRes, StaticArg},
};
use encase::ShaderType;
use wgpu::{BufferBindingType, ShaderStages};

#[derive(Debug, Default, Copy, Clone, ShaderType)]
pub struct GlobalsData {
    pub frame: u32,
    pub time: f32,
    pub delta_time: f32,
    _padding: f32,
}

#[derive(Clone)]
pub struct GlobalLayout(BindGroupLayout);
impl GlobalLayout {
    pub fn new(device: &RenderDevice) -> Self {
        let layout = BindGroupLayoutBuilder::new()
            .with_buffer(
                0,
                ShaderStages::all(),
                BufferBindingType::Uniform,
                Some(GlobalsData::min_size()),
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

        Self(layout)
    }

    pub fn inner(&self) -> &BindGroupLayout {
        &self.0
    }
}

impl std::ops::Deref for GlobalLayout {
    type Target = BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BindGroupLayout> for GlobalLayout {
    fn from(layout: BindGroupLayout) -> Self {
        Self(layout)
    }
}

impl Resource for GlobalLayout {}

pub struct Globals<V: Send + 'static> {
    binding: BindGroup,
    layout: GlobalLayout,
    buffer: UniformBuffer<GlobalsData>,
    views: UniformBufferArray<RenderViewData>,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: Send + 'static> Globals<V> {
    pub fn new(device: &RenderDevice, layout: &GlobalLayout, data: GlobalsData) -> Self {
        let buffer = UniformBuffer::with_buffer(device, data);
        let mut views = UniformBufferArray::<RenderViewData>::aligned(device);
        views.push(&RenderViewData::default());
        views.update(device);

        let entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.binding().unwrap(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: views.binding().unwrap(),
            },
        ];

        Self {
            binding: BindGroup::create(device, layout, &entries, ()),
            layout: layout.clone(),
            buffer,
            views,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(&self) -> &GlobalLayout {
        &self.layout
    }

    pub fn buffer(&self) -> &UniformBuffer<GlobalsData> {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut UniformBuffer<GlobalsData> {
        &mut self.buffer
    }

    pub fn views(&self) -> &UniformBufferArray<RenderViewData> {
        &self.views
    }

    pub fn views_mut(&mut self) -> &mut UniformBufferArray<RenderViewData> {
        &mut self.views
    }
}

impl<V: Send + 'static> Resource for Globals<V> {}

impl<V: Send + 'static> RenderResourceExtractor for Globals<V> {
    type Resource = Globals<V>;

    type Arg = StaticArg<'static, (ReadRes<RenderDevice>, ReadRes<GlobalLayout>)>;

    fn extract(
        arg: ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Resource, crate::ExtractError> {
        let (device, layout) = arg.inner();

        let data = GlobalsData::default();
        Ok(Self::new(device, layout, data))
    }
}
