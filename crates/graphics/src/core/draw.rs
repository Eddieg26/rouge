use crate::resources::{
    binding::{BindGroup, BindGroupId},
    buffer::{BufferSlice, BufferSliceId, IndexSlice},
    pipeline::{RenderPipeline, RenderPipelineId},
};
use bytemuck::{Pod, Zeroable};
use ecs::{
    core::{entity::Entity, IndexMap},
    system::SystemArg,
};
use glam::{Mat4, Vec4};
use std::{collections::HashMap, hash::Hash};
use wgpu::IndexFormat;

pub trait Draw: Sized + Send + Sync + 'static {
    fn entity(&self) -> Entity;
}

pub trait BatchDraw: Draw {
    type Key: Copy + Eq + Hash + Ord + Send + Sync + 'static;
    type Data: Pod + 'static;

    fn key(&self) -> Self::Key;
    fn data(&self) -> Self::Data;
    fn can_batch(&self) -> bool;
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RenderViewData {
    pub position: Vec4,
    pub world: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
    pub frustum: [Vec4; 6],
}

pub struct RenderView<V: 'static> {
    pub data: RenderViewData,
    view: V,
}

impl<V: 'static> std::ops::Deref for RenderView<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

pub struct DrawCalls<D: Draw> {
    calls: Vec<D>,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn add(&mut self, call: D) {
        self.calls.push(call);
    }

    pub fn iter(&self) -> impl Iterator<Item = &D> {
        self.calls.iter()
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl<D: Draw + Ord> DrawCalls<D> {
    pub fn sort(&mut self) {
        self.calls.sort_unstable();
    }
}

pub struct BatchDrawCalls<D: BatchDraw> {
    calls: Vec<D>,
    unbatched_indices: Vec<u32>,
    batches: IndexMap<D::Key, Vec<u32>>,
}

impl<D: BatchDraw> BatchDrawCalls<D> {
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            unbatched_indices: Vec::new(),
            batches: IndexMap::new(),
        }
    }

    pub fn add(&mut self, call: D) {
        if call.can_batch() {
            let key = call.key();
            if let Some(batch) = self.batches.get_mut(&key) {
                batch.push(self.calls.len() as u32);
            } else {
                self.batches.insert(key, vec![self.calls.len() as u32]);
            }
        } else {
            self.unbatched_indices.push(self.calls.len() as u32);
        }

        self.calls.push(call);
    }

    pub fn iter(&self) -> impl Iterator<Item = &D> {
        self.calls.iter()
    }

    pub fn unbatched_indices(&self) -> &[u32] {
        &self.unbatched_indices
    }

    pub fn batch_indices(&self, key: &D::Key) -> Option<&[u32]> {
        self.batches.get(key).map(|v| v.as_slice())
    }

    pub fn batches(&self) -> impl Iterator<Item = (&D::Key, impl Iterator<Item = &D>)> {
        self.batches
            .iter()
            .map(|(k, v)| (k, v.iter().map(|i| &self.calls[*i as usize])))
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn clear(&mut self) {
        self.calls.clear();
        self.unbatched_indices.clear();
        self.batches.clear();
    }

    pub fn sort(&mut self) {
        self.calls.sort_unstable_by_key(|c| c.key());
    }
}

impl<D: BatchDraw + Ord> BatchDrawCalls<D> {
    pub fn sort_batches(&mut self) {
        self.unbatched_indices.sort_by(|a, b| {
            self.calls[*a as usize]
                .key()
                .cmp(&self.calls[*b as usize].key())
        });
    }
}

pub struct RenderState<'a> {
    pass: &'a mut wgpu::RenderPass<'a>,
    vertex_buffers: HashMap<u32, BufferSliceId>,
    index_buffer: Option<BufferSliceId>,
    bind_groups: HashMap<u32, (BindGroupId, Vec<u32>)>,
    pipeline: Option<RenderPipelineId>,
}

impl<'a> RenderState<'a> {
    pub fn new(pass: &'a mut wgpu::RenderPass<'a>) -> Self {
        Self {
            pass,
            vertex_buffers: HashMap::new(),
            index_buffer: None,
            bind_groups: HashMap::new(),
            pipeline: None,
        }
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, slice: BufferSlice<'_>) {
        match self.vertex_buffers.get(&slot) {
            Some(id) if id != &slice.id() => {
                self.pass.set_vertex_buffer(slot, *slice);
                self.vertex_buffers.insert(slot, slice.id());
            }
            None => {
                self.pass.set_vertex_buffer(slot, *slice);
                self.vertex_buffers.insert(slot, slice.id());
            }
            _ => (),
        }
    }

    pub fn set_index_buffer(&mut self, slice: IndexSlice<'_>) {
        match self.index_buffer.as_ref() {
            Some(id) if id != &slice.id() => {
                self.pass.set_index_buffer(*slice, IndexFormat::Uint32);
                self.index_buffer = Some(slice.id());
            }
            None => {
                self.pass.set_index_buffer(*slice, IndexFormat::Uint32);
                self.index_buffer = Some(slice.id());
            }
            _ => (),
        }
    }

    pub fn set_bind_group<D: Send + Sync + 'static>(
        &mut self,
        group: u32,
        bind_group: &BindGroup<D>,
        offset: &[u32],
    ) {
        match self.bind_groups.get(&group) {
            Some((id, bindings)) if id != &bind_group.id() || bindings.as_slice() == offset => {
                self.pass
                    .set_bind_group(group, Some(bind_group.inner()), offset);
                self.bind_groups
                    .insert(group, (bind_group.id(), offset.to_vec()));
            }
            None => {
                self.pass
                    .set_bind_group(group, Some(bind_group.inner()), offset);
                self.bind_groups
                    .insert(group, (bind_group.id(), offset.to_vec()));
            }
            _ => (),
        }
    }

    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        if self.pipeline.as_ref() != Some(&pipeline.id()) {
            self.pass.set_pipeline(pipeline);
            self.pipeline = Some(pipeline.id());
        }
    }

    pub fn clear(&mut self) {
        self.vertex_buffers.clear();
        self.index_buffer = None;
        self.bind_groups.clear();
        self.pipeline = None;
    }
}

pub trait DrawSystem {
    type Draw: Draw;
    type View: 'static;
    type Arg: SystemArg;

    fn draw(
        &mut self,
        view: &RenderView<Self::View>,
        draw_calls: &DrawCalls<Self::Draw>,
        arg: Self::Arg,
    );
}

pub trait BatchDrawSystem {
    type Draw: BatchDraw;
    type View: 'static;
    type Arg: SystemArg;

    fn draw(
        &mut self,
        view: &RenderView<Self::View>,
        draw_calls: &BatchDrawCalls<Self::Draw>,
        arg: Self::Arg,
    );
}
