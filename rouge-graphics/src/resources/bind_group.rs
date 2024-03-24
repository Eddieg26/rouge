use crate::core::ty::shader::IntoBindGroupLayout;
use rouge_core::ResourceId;
use rouge_ecs::{macros::Resource, storage::sparse::SparseMap};
use std::collections::HashMap;

pub struct BindGroups {
    layout: wgpu::BindGroupLayout,
    groups: SparseMap<ResourceId, wgpu::BindGroup>,
}

impl BindGroups {
    pub fn new(layout: wgpu::BindGroupLayout) -> Self {
        Self {
            layout,
            groups: SparseMap::new(),
        }
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn insert(&mut self, id: ResourceId, group: wgpu::BindGroup) {
        self.groups.insert(id, group);
    }

    pub fn get(&self, id: &ResourceId) -> Option<&wgpu::BindGroup> {
        self.groups.get(id)
    }

    pub fn get_mut(&mut self, id: &ResourceId) -> Option<&mut wgpu::BindGroup> {
        self.groups.get_mut(id)
    }

    pub fn remove(&mut self, id: &ResourceId) -> Option<wgpu::BindGroup> {
        self.groups.remove(id)
    }
}

#[derive(Resource)]
pub struct BindGroupLayouts {
    groups: HashMap<ResourceId, BindGroups>,
}

impl BindGroupLayouts {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        device: &wgpu::Device,
        id: ResourceId,
        layout: impl IntoBindGroupLayout,
    ) {
        let layout = layout.into_bind_group_layout(device);
        self.groups.insert(id, BindGroups::new(layout));
    }

    pub fn get(&self, id: &ResourceId) -> Option<&BindGroups> {
        self.groups.get(id)
    }

    pub fn get_mut(&mut self, id: &ResourceId) -> Option<&mut BindGroups> {
        self.groups.get_mut(id)
    }

    pub fn remove(&mut self, id: &ResourceId) -> Option<BindGroups> {
        self.groups.remove(id)
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.groups.contains_key(id)
    }
}
