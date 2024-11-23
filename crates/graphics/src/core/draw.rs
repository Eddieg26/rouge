use super::Color;
use crate::resource::{
    binding::{BindGroup, BindGroupId},
    buffer::{Buffer, BufferSlice, BufferSliceId, IndexSlice},
    pipeline::{RenderPipeline, RenderPipelineId},
};
use bytemuck::{Pod, Zeroable};
use ecs::{
    core::{entity::Entity, IndexMap},
    system::SystemArg,
};
use glam::{Mat4, Vec4};
use spatial::rect::Rect;
use std::{collections::HashMap, hash::Hash, ops::Range};
use wgpu::{IndexFormat, QuerySet, RenderBundle, ShaderStages};

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
    pub viewport: Viewport,
    view: V,
}

impl<V: 'static> std::ops::Deref for RenderView<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Viewport {
    pub fn new(x: f32, y: f32, width: f32, height: f32, depth: Range<f32>) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: depth.start,
            max_depth: depth.end,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl From<(Rect, Range<f32>)> for Viewport {
    fn from((rect, depth): (Rect, Range<f32>)) -> Self {
        Self::new(rect.x(), rect.y(), rect.width(), rect.height(), depth)
    }
}

impl From<(&Rect, Range<f32>)> for Viewport {
    fn from((rect, depth): (&Rect, Range<f32>)) -> Self {
        Self::new(rect.x(), rect.y(), rect.width(), rect.height(), depth)
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
            self.clear();
            self.pass.set_pipeline(pipeline);
            self.pipeline = Some(pipeline.id());
        }
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        let Viewport {
            x,
            y,
            width,
            height,
            min_depth,
            max_depth,
        } = viewport;

        self.pass
            .set_viewport(x, y, width, height, min_depth, max_depth);
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.pass.set_scissor_rect(x, y, width, height);
    }

    pub fn set_blend_constant(&mut self, color: Color) {
        self.pass.set_blend_constant(color.into());
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.pass.set_push_constants(stages, offset, data);
    }

    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.pass.set_stencil_reference(reference);
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.pass.draw(vertices, instances);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, start: i32, instances: Range<u32>) {
        self.pass.draw_indexed(indices, start, instances);
    }

    pub fn draw_indirect(&mut self, buffer: &Buffer, offset: u64) {
        self.pass.draw_indirect(buffer.inner(), offset);
    }

    pub fn draw_indexed_indirect(&mut self, buffer: &Buffer, offset: u64) {
        self.pass.draw_indexed_indirect(buffer.inner(), offset);
    }

    pub fn multi_draw_indirect(&mut self, buffer: &Buffer, offset: u64, count: u32) {
        self.pass.multi_draw_indirect(buffer.inner(), offset, count);
    }

    pub fn multi_draw_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        count_buffer: &Buffer,
        count_offset: u64,
        max_count: u32,
    ) {
        self.pass.multi_draw_indirect_count(
            buffer.inner(),
            offset,
            count_buffer.inner(),
            count_offset,
            max_count,
        );
    }

    pub fn multi_draw_indexed_indirect(&mut self, buffer: &Buffer, offset: u64, count: u32) {
        self.pass
            .multi_draw_indexed_indirect(buffer.inner(), offset, count);
    }

    pub fn multi_draw_indexed_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        count_buffer: &Buffer,
        count_offset: u64,
        max_count: u32,
    ) {
        self.pass.multi_draw_indexed_indirect_count(
            buffer.inner(),
            offset,
            count_buffer.inner(),
            count_offset,
            max_count,
        );
    }

    pub fn push_debug_group(&mut self, label: &str) {
        self.pass.push_debug_group(label);
    }

    pub fn pop_debug_group(&mut self) {
        self.pass.pop_debug_group();
    }

    pub fn begin_occlusion_query(&mut self, query_index: u32) {
        self.pass.begin_occlusion_query(query_index);
    }

    pub fn end_occlusion_query(&mut self) {
        self.pass.end_occlusion_query();
    }

    pub fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, query_index: u32) {
        self.pass
            .begin_pipeline_statistics_query(query_set, query_index);
    }

    pub fn end_pipeline_statistics_query(&mut self) {
        self.pass.end_pipeline_statistics_query();
    }

    pub fn execute_bundles<I>(&mut self, render_bundles: I)
    where
        I: IntoIterator<Item = &'a RenderBundle>,
    {
        self.pass.execute_bundles(render_bundles);
    }

    pub fn insert_debug_marker(&mut self, label: &str) {
        self.pass.insert_debug_marker(label);
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, query_index: u32) {
        self.pass.write_timestamp(query_set, query_index);
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
        state: &mut RenderState,
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
        state: &mut RenderState,
        arg: Self::Arg,
    );
}
