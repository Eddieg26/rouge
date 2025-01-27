use super::{
    LightBinding, Material, MaterialInstance, MaterialLight, MaterialMesh, MaterialMeshData,
    MaterialPipeline, MaterialRenderView, MaterialRenderViews, MaterialView, ModelUniform,
    RenderView,
};
use crate::{
    extract::RenderAssets,
    renderer::state::RenderState,
    resource::{BufferArrayIndex, Mesh, RenderMesh, SubMesh, VertexBufferArray},
};
use asset::AssetRef;
use ecs::Entity;
use spatial::Aabb;
use std::{collections::HashMap, hash::Hash, ops::Range};

pub struct BatchKey<M: Material> {
    pub material: AssetRef<M>,
    pub mesh: AssetRef<Mesh>,
    pub sub_mesh: Option<SubMesh>,
}

impl<M: Material> Copy for BatchKey<M> {}
impl<M: Material> Clone for BatchKey<M> {
    fn clone(&self) -> Self {
        BatchKey {
            material: self.material.clone(),
            mesh: self.mesh.clone(),
            sub_mesh: self.sub_mesh,
        }
    }
}

impl<M: Material> Eq for BatchKey<M> {}
impl<M: Material> PartialEq for BatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material
            && self.mesh == other.mesh
            && self.sub_mesh == other.sub_mesh
    }
}

impl<M: Material> Hash for BatchKey<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.material.hash(state);
        self.mesh.hash(state);
        self.sub_mesh.hash(state);
    }
}

impl<M: Material> BatchKey<M> {
    pub fn new(material: AssetRef<M>, mesh: AssetRef<Mesh>, sub_mesh: Option<SubMesh>) -> Self {
        BatchKey {
            material,
            mesh,
            sub_mesh,
        }
    }
}

pub trait Draw: Send + Sync + 'static {
    type Material: Material;

    const BATCH: bool = true;

    fn entity(&self) -> ecs::Entity;
    fn material(&self) -> AssetRef<Self::Material>;
    fn mesh(&self) -> AssetRef<Mesh>;
    fn submesh(&self) -> Option<SubMesh> {
        None
    }
    fn data(&self) -> MaterialMeshData<Self::Material>;
}

fn prepare_draw_calls<D: Draw>(
    view_draw_calls: &mut ViewDrawCalls<D>,
    calls: &mut DrawCalls<D>,
    buffer: &mut MaterialMesh<D::Material>,
) {
    for visible_calls in view_draw_calls.views.values_mut() {
        for batch in visible_calls.batched.values_mut() {
            for index in &batch.indices {
                let call = &calls.calls[index.index as usize];
                let buffer_index = buffer.buffer.push(call.data());
                match &mut batch.instances {
                    Some(instances) => instances.end = buffer_index.index + 1,
                    None => batch.instances = Some(buffer_index.index..buffer_index.index + 1),
                }
            }
        }

        for call_index in visible_calls.unbatched.iter_mut() {
            let call = &calls.calls[call_index.index.index as usize];
            let buffer_index = buffer.buffer.push(call.data());
            call_index.start = buffer_index.index;
        }
    }
}

fn render_draw_calls<D: Draw>(
    state: &mut RenderState,
    views: &MaterialRenderViews<D::Material>,
    calls: &DrawCalls<D>,
    view_draw_calls: &mut ViewDrawCalls<D>,
    pipeline: &MaterialPipeline<D::Material>,
    meshes: &RenderAssets<RenderMesh>,
    materials: RenderAssets<MaterialInstance<D::Material>>,
    view_binding: &MaterialView<D::Material>,
    light_binding: &MaterialLight<D::Material>,
    mesh_binding: &MaterialMesh<D::Material>,
) {
    for (view_entity, visible_calls) in view_draw_calls.views.iter() {
        let Some(view) = views.get(view_entity) else {
            continue;
        };

        state.set_pipeline(&pipeline);

        state.set_bind_group(0, &view_binding.binding, &[*view.dynamic_offset]);

        if let Some(bind_group) = light_binding.bind_group() {
            state.set_bind_group(3, bind_group, &[]);
        }

        for (key, batch) in visible_calls.batched.iter() {
            let Some(material) = materials.get(&key.material) else {
                continue;
            };

            let Some(mesh) = meshes.get(&key.mesh) else {
                continue;
            };

            let instances = batch.instances.clone().unwrap_or(0..1);

            let Some(slice) = mesh_binding.buffer.slice(..) else {
                continue;
            };

            state.set_vertex_buffer(0, slice);

            state.set_bind_group(2, material.bind_group(), &[]);

            draw_instances::<D>(state, mesh, key.sub_mesh, instances);
        }

        for call_index in visible_calls.unbatched.iter() {
            let call = &calls.calls[call_index.index.index as usize];
            let Some(material) = materials.get(&call.material()) else {
                continue;
            };

            let Some(mesh) = meshes.get(&call.mesh()) else {
                continue;
            };

            let Some(slice) = mesh_binding.buffer.slice(..) else {
                continue;
            };

            state.set_vertex_buffer(0, slice);

            state.set_bind_group(2, material.bind_group(), &[]);

            draw_instances::<D>(state, mesh, call.submesh(), 0..1);
        }
    }
}

fn draw_instances<D: Draw>(
    state: &mut RenderState,
    mesh: &RenderMesh,
    sub_mesh: Option<SubMesh>,
    instances: Range<u32>,
) {
    if let Some(index_buffer) = mesh.index_buffer() {
        let (base_vertex, indices) = match sub_mesh {
            Some(sub_mesh) => {
                let base_vertex = sub_mesh.start_vertex as i32;
                let start_index = sub_mesh.start_index as u32;
                let index_count = sub_mesh.index_count as u32;

                (base_vertex, start_index..start_index + index_count)
            }
            None => (0, 0..index_buffer.len() as u32),
        };

        state.set_index_buffer(index_buffer.slice(..));

        state.draw_indexed(indices, base_vertex, instances);
    } else {
        let vertices = match sub_mesh {
            Some(sub_mesh) => {
                let start = sub_mesh.start_vertex as u32;
                let count = sub_mesh.vertex_count as u32;

                start..start + count
            }
            None => 0..mesh.vertex_count() as u32,
        };

        state.draw(vertices, instances);
    }
}

pub trait DrawSorter<D: Draw> {
    type SortKey: Copy + PartialOrd + PartialEq;

    fn get_sort_key(&self, view: &MaterialRenderView<D::Material>) -> Self::SortKey;
}

pub trait DrawCuller<D: Draw> {
    
}

pub struct MeshCullData {
    bounds: Aabb,
}

pub trait CulledDraw: Draw {}

pub struct DrawCalls<D>
where
    D: Draw,
{
    calls: Vec<D>,
}

impl<D: Draw> DrawCalls<D> {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn calls(&self) -> &[D] {
        &self.calls
    }

    pub fn push(&mut self, call: D) {
        self.calls.push(call);
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

pub struct DrawCallIndex<D: Draw> {
    pub index: BufferArrayIndex<MaterialMeshData<D::Material>>,
    pub start: u32,
}

pub struct DrawCallBatch<D: Draw> {
    pub indices: Vec<BufferArrayIndex<MaterialMeshData<D::Material>>>,
    pub instances: Option<Range<u32>>,
}

pub struct VisibleDrawCalls<D: Draw> {
    unbatched: Vec<DrawCallIndex<D>>,
    batched: HashMap<BatchKey<D::Material>, DrawCallBatch<D>>,
}

pub struct ViewDrawCalls<D: Draw> {
    views: HashMap<Entity, VisibleDrawCalls<D>>,
}
