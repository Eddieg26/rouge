use bytemuck::{Pod, Zeroable};
use ecs::{
    core::{entity::Entity, IndexMap},
    system::SystemArg,
};
use glam::{Mat4, Vec4};
use std::hash::Hash;

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
