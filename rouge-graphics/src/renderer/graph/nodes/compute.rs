use super::{GraphNode, RenderPhase};
use crate::{
    renderer::graph::{context::RenderContext, resources::GraphResources},
    resources::shader::Shader,
};
use rouge_asset::{storage::Assets, AssetId};
use rouge_ecs::{meta::AccessMeta, ArgItem, SystemArg, World};

pub trait ComputeExecutor: Send + Sync + 'static {
    type Arg: SystemArg;

    fn shader(&self) -> AssetId;
    fn access(&self) -> Vec<AccessMeta>;
    fn execute(
        &self,
        resources: &GraphResources,
        arg: ArgItem<'_, Self::Arg>,
        pass: wgpu::ComputePass,
    );
}

pub struct ComputeExecutorInstance {
    execute: Box<dyn Fn(&World, &GraphResources, wgpu::ComputePass) + Send + Sync>,
    shader: AssetId,
    access: Vec<AccessMeta>,
}

impl ComputeExecutorInstance {
    pub fn new<'a, E: ComputeExecutor>(executor: E) -> Self {
        let mut access = executor.access();
        access.append(&mut E::Arg::metas());

        Self {
            shader: executor.shader(),
            access,
            execute: Box::new(move |world, resources, pass| {
                let arg = E::Arg::get(world);
                executor.execute(resources, arg, pass);
            }),
        }
    }

    pub fn shader(&self) -> AssetId {
        self.shader
    }

    pub fn access(&self) -> &[AccessMeta] {
        &self.access
    }
    
    pub fn execute(&self, world: &World, resources: &GraphResources, pass: wgpu::ComputePass) {
        (self.execute)(world, resources, pass);
    }
}

pub struct ComputePass {
    phase: RenderPhase,
    executor: ComputeExecutorInstance,
    pipeline: Option<wgpu::ComputePipeline>,
}

impl ComputePass {
    pub fn new(executor: impl ComputeExecutor) -> Self {
        Self {
            phase: RenderPhase::Process,
            executor: ComputeExecutorInstance::new(executor),
            pipeline: None,
        }
    }

    pub fn set_phase(mut self, phase: RenderPhase) -> Self {
        self.phase = phase;
        self
    }
}

impl GraphNode for ComputePass {
    fn prepare(&mut self, ctx: RenderContext) {
        let device = ctx.device();
        let shader = ctx
            .system_arg::<&Assets<Shader>>()
            .get(&self.executor.shader())
            .expect("Shader not found");
        let compute_meta = shader.compute().expect("No compute entry point");
        let bind_group_layouts = compute_meta.create_layouts(device);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: bind_group_layouts
                .iter()
                .map(|l| l)
                .collect::<Vec<_>>()
                .as_slice(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: shader.module(),
            entry_point: compute_meta.entry(),
        });

        self.pipeline = Some(pipeline);
    }

    fn execute(&self, ctx: RenderContext) {
        let mut encoder = ctx.create_encoder();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        let pipeline = self.pipeline.as_ref().expect("Pipeline not prepared");
        pass.set_pipeline(pipeline);

        self.executor
            .execute(ctx.system_arg::<&World>(), ctx.resources(), pass);

        ctx.submit(encoder);
    }

    fn phase(&self) -> RenderPhase {
        self.phase
    }
}
