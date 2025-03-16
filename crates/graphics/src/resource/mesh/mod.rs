use super::buffer::{IndexBuffer, Indices, VertexBuffer};
use crate::{
    core::{Color, RenderDevice},
    extract::{
        AssetUsage, ExtractError, ReadWrite, RenderAsset, RenderAssetExtractor, RenderAssets,
    },
};
use asset::{asset::Asset, AssetId, AssetRef};
use ecs::system::{unlifetime::ReadRes, ArgItem};
use spatial::bounds::BoundingBox;
use std::{hash::Hash, ops::Range};
use wgpu::BufferUsages;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum MeshTopology {
    PointList = 0,
    LineList = 1,
    LineStrip = 2,
    #[default]
    TriangleList = 3,
    TriangleStrip = 4,
}

impl Into<wgpu::PrimitiveTopology> for MeshTopology {
    fn into(self) -> wgpu::PrimitiveTopology {
        match self {
            MeshTopology::PointList => wgpu::PrimitiveTopology::PointList,
            MeshTopology::LineList => wgpu::PrimitiveTopology::LineList,
            MeshTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            MeshTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            MeshTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

impl From<wgpu::PrimitiveTopology> for MeshTopology {
    fn from(topology: wgpu::PrimitiveTopology) -> Self {
        match topology {
            wgpu::PrimitiveTopology::PointList => MeshTopology::PointList,
            wgpu::PrimitiveTopology::LineList => MeshTopology::LineList,
            wgpu::PrimitiveTopology::LineStrip => MeshTopology::LineStrip,
            wgpu::PrimitiveTopology::TriangleList => MeshTopology::TriangleList,
            wgpu::PrimitiveTopology::TriangleStrip => MeshTopology::TriangleStrip,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeshAttribute {
    Position(Vec<glam::Vec3>),
    Normal(Vec<glam::Vec3>),
    TexCoord0(Vec<glam::Vec2>),
    TexCoord1(Vec<glam::Vec2>),
    Tangent(Vec<glam::Vec4>),
    Color(Vec<Color>),
}

impl MeshAttribute {
    pub fn kind(&self) -> MeshAttributeKind {
        match self {
            MeshAttribute::Position(_) => MeshAttributeKind::Position,
            MeshAttribute::Normal(_) => MeshAttributeKind::Normal,
            MeshAttribute::TexCoord0(_) => MeshAttributeKind::TexCoord0,
            MeshAttribute::TexCoord1(_) => MeshAttributeKind::TexCoord1,
            MeshAttribute::Tangent(_) => MeshAttributeKind::Tangent,
            MeshAttribute::Color(_) => MeshAttributeKind::Color,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MeshAttribute::Position(v) => v.len(),
            MeshAttribute::Normal(v) => v.len(),
            MeshAttribute::TexCoord0(v) => v.len(),
            MeshAttribute::TexCoord1(v) => v.len(),
            MeshAttribute::Tangent(v) => v.len(),
            MeshAttribute::Color(v) => v.len(),
        }
    }

    pub fn extend(&mut self, other: &Self) {
        match (self, other) {
            (MeshAttribute::Position(a), MeshAttribute::Position(b)) => a.extend_from_slice(b),
            (MeshAttribute::Normal(a), MeshAttribute::Normal(b)) => a.extend_from_slice(b),
            (MeshAttribute::TexCoord0(a), MeshAttribute::TexCoord0(b)) => a.extend_from_slice(b),
            (MeshAttribute::TexCoord1(a), MeshAttribute::TexCoord1(b)) => a.extend_from_slice(b),
            (MeshAttribute::Tangent(a), MeshAttribute::Tangent(b)) => a.extend_from_slice(b),
            (MeshAttribute::Color(a), MeshAttribute::Color(b)) => a.extend_from_slice(b),
            _ => (),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MeshAttribute::Position(v) => v.is_empty(),
            MeshAttribute::Normal(v) => v.is_empty(),
            MeshAttribute::TexCoord0(v) => v.is_empty(),
            MeshAttribute::TexCoord1(v) => v.is_empty(),
            MeshAttribute::Tangent(v) => v.is_empty(),
            MeshAttribute::Color(v) => v.is_empty(),
        }
    }

    pub fn data(&self, range: Range<usize>) -> &[u8] {
        match self {
            MeshAttribute::Position(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Normal(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::TexCoord0(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::TexCoord1(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Tangent(v) => bytemuck::cast_slice(&v[range]),
            MeshAttribute::Color(v) => bytemuck::cast_slice(&v[range]),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            MeshAttribute::Position(_) => std::mem::size_of::<glam::Vec3>(),
            MeshAttribute::Normal(_) => std::mem::size_of::<glam::Vec3>(),
            MeshAttribute::TexCoord0(_) => std::mem::size_of::<glam::Vec2>(),
            MeshAttribute::TexCoord1(_) => std::mem::size_of::<glam::Vec2>(),
            MeshAttribute::Tangent(_) => std::mem::size_of::<glam::Vec4>(),
            MeshAttribute::Color(_) => std::mem::size_of::<Color>(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            MeshAttribute::Position(v) => v.clear(),
            MeshAttribute::Normal(v) => v.clear(),
            MeshAttribute::TexCoord0(v) => v.clear(),
            MeshAttribute::TexCoord1(v) => v.clear(),
            MeshAttribute::Tangent(v) => v.clear(),
            MeshAttribute::Color(v) => v.clear(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeshAttributeKind {
    Position,
    Normal,
    TexCoord0,
    TexCoord1,
    Tangent,
    Color,
}

impl MeshAttributeKind {
    pub fn size(&self) -> usize {
        match self {
            MeshAttributeKind::Position => std::mem::size_of::<glam::Vec3>(),
            MeshAttributeKind::Normal => std::mem::size_of::<glam::Vec3>(),
            MeshAttributeKind::TexCoord0 => std::mem::size_of::<glam::Vec2>(),
            MeshAttributeKind::TexCoord1 => std::mem::size_of::<glam::Vec2>(),
            MeshAttributeKind::Tangent => std::mem::size_of::<glam::Vec4>(),
            MeshAttributeKind::Color => std::mem::size_of::<Color>(),
        }
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            MeshAttributeKind::Position => wgpu::VertexFormat::Float32x3,
            MeshAttributeKind::Normal => wgpu::VertexFormat::Float32x3,
            MeshAttributeKind::TexCoord0 => wgpu::VertexFormat::Float32x2,
            MeshAttributeKind::TexCoord1 => wgpu::VertexFormat::Float32x2,
            MeshAttributeKind::Tangent => wgpu::VertexFormat::Float32x4,
            MeshAttributeKind::Color => wgpu::VertexFormat::Float32x4,
        }
    }
}

impl Iterator for MeshAttributeKind {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MeshAttributeKind::Position => Some(MeshAttributeKind::Normal),
            MeshAttributeKind::Normal => Some(MeshAttributeKind::TexCoord0),
            MeshAttributeKind::TexCoord0 => Some(MeshAttributeKind::TexCoord1),
            MeshAttributeKind::TexCoord1 => Some(MeshAttributeKind::Tangent),
            MeshAttributeKind::Tangent => Some(MeshAttributeKind::Color),
            MeshAttributeKind::Color => None,
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, PartialEq, Eq)]
    pub struct MeshDirty: u32 {
        const POSITION = 1 << 1;
        const NORMAL = 1 << 2;
        const TANGENT =  1 << 3;
        const TEXCOORD0 = 1 << 4;
        const TEXCOORD1 = 1 << 5;
        const COLOR = 1 << 6;
        const INDICES = 1 << 7;
        const BOUNDS = 1 << 8;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SubMesh {
    pub start_vertex: u64,
    pub vertex_count: u64,
    pub start_index: u64,
    pub index_count: u64,
}

impl SubMesh {
    pub fn new(start_vertex: u64, vertex_count: u64, start_index: u64, index_count: u64) -> Self {
        Self {
            start_vertex,
            vertex_count,
            start_index,
            index_count,
        }
    }
}

impl From<&Mesh> for SubMesh {
    fn from(mesh: &Mesh) -> Self {
        Self {
            start_vertex: 0,
            vertex_count: mesh.vertex_count() as u64,
            start_index: 0,
            index_count: mesh.index_count() as u64,
        }
    }
}

impl Asset for SubMesh {}

#[derive(Clone, Copy)]
pub struct MeshId {
    pub id: AssetRef<Mesh>,
    pub sub: Option<SubMesh>,
}

impl From<AssetRef<Mesh>> for MeshId {
    fn from(id: AssetRef<Mesh>) -> Self {
        Self { id, sub: None }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    topology: MeshTopology,
    attributes: Vec<MeshAttribute>,
    indices: Option<Indices>,
    bounds: BoundingBox,
    read_write: ReadWrite,
    sub_meshes: Vec<SubMesh>,

    #[serde(skip)]
    dirty: MeshDirty,
}

impl Mesh {
    pub fn new(topology: MeshTopology) -> Self {
        Self {
            topology,
            attributes: Vec::new(),
            indices: None,
            bounds: BoundingBox::ZERO,
            read_write: ReadWrite::Disabled,
            sub_meshes: Vec::new(),
            dirty: MeshDirty::empty(),
        }
    }

    pub fn with_attribute(mut self, attribute: MeshAttribute) -> Self {
        self.add_attribute(attribute);
        self
    }

    pub fn with_read_write(mut self, read_write: ReadWrite) -> Self {
        self.read_write = read_write;
        self
    }

    pub fn topology(&self) -> MeshTopology {
        self.topology
    }

    pub fn attributes(&self) -> &[MeshAttribute] {
        &self.attributes
    }

    pub fn attribute(&self, kind: MeshAttributeKind) -> Option<&MeshAttribute> {
        self.attribute_index(kind).map(|i| &self.attributes[i])
    }

    pub fn attribute_mut(&mut self, kind: MeshAttributeKind) -> Option<&mut MeshAttribute> {
        match self.attribute_index(kind) {
            Some(i) => {
                self.attribute_dirty(kind);
                Some(&mut self.attributes[i])
            }
            None => None,
        }
    }

    pub fn sub_meshes(&self) -> &[SubMesh] {
        &self.sub_meshes
    }

    pub fn sub_mesh(&self, index: usize) -> Option<&SubMesh> {
        self.sub_meshes.get(index)
    }

    pub fn dirty(&self) -> MeshDirty {
        self.dirty
    }

    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        let indices = self.indices.as_mut();

        if indices.is_some() {
            self.dirty |= MeshDirty::INDICES;
        }

        indices
    }

    pub fn bounds(&self) -> BoundingBox {
        self.bounds
    }

    pub fn read_write(&self) -> ReadWrite {
        self.read_write
    }

    pub fn add_attribute(&mut self, attribute: MeshAttribute) {
        let kind = attribute.kind();
        match self.attribute_index(kind) {
            Some(i) => self.attributes[i] = attribute,
            None => self.attributes.push(attribute),
        }

        self.attribute_dirty(kind);
    }

    pub fn remove_attribute(&mut self, kind: MeshAttributeKind) -> Option<MeshAttribute> {
        let removed = self
            .attribute_index(kind)
            .map(|i| self.attributes.remove(i));

        self.attribute_dirty(kind);

        removed
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
        self.dirty |= MeshDirty::INDICES;
    }

    pub fn add_indices(&mut self, indices: Indices) {
        match self.indices {
            Some(ref mut i) => i.extend(&indices),
            None => self.indices = Some(indices),
        }
    }

    pub fn attribute_index(&self, kind: MeshAttributeKind) -> Option<usize> {
        self.attributes.iter().position(|a| a.kind() == kind)
    }

    pub fn add_sub_mesh(&mut self, sub_mesh: SubMesh) {
        self.sub_meshes.push(sub_mesh);
    }

    pub fn remove_sub_mesh(&mut self, index: usize) -> SubMesh {
        self.sub_meshes.remove(index)
    }

    pub fn clear(&mut self) {
        for attribute in &mut self.attributes {
            attribute.clear();
        }

        self.indices = None;
        self.dirty = MeshDirty::all()
    }

    pub fn vertex_count(&self) -> u64 {
        if self.attributes.is_empty() {
            return 0;
        }

        self.attributes
            .iter()
            .fold(usize::MAX, |len, curr| len.min(curr.len())) as u64
    }

    pub fn index_count(&self) -> usize {
        self.indices.as_ref().map_or(0, |i| i.len())
    }

    pub fn calculate_bounds(&mut self) {
        let bounds_dirty = self.dirty.contains(MeshDirty::BOUNDS);

        match (bounds_dirty, self.attribute(MeshAttributeKind::Position)) {
            (true, Some(MeshAttribute::Position(positions))) => {
                self.bounds = BoundingBox::from(positions.as_slice());
                self.dirty.remove(MeshDirty::BOUNDS);
            }
            _ => (),
        }
    }

    pub fn attribute_data(&self, kind: MeshAttributeKind, range: Range<usize>) -> &[u8] {
        self.attribute(kind).map_or(&[], |a| a.data(range))
    }

    pub fn attribute_dirty(&mut self, attribute: MeshAttributeKind) {
        match attribute {
            MeshAttributeKind::Position => self.dirty |= MeshDirty::POSITION | MeshDirty::BOUNDS,
            MeshAttributeKind::Normal => self.dirty |= MeshDirty::NORMAL,
            MeshAttributeKind::Tangent => self.dirty |= MeshDirty::TANGENT,
            MeshAttributeKind::TexCoord0 => self.dirty |= MeshDirty::TEXCOORD0,
            MeshAttributeKind::TexCoord1 => self.dirty |= MeshDirty::TEXCOORD1,
            MeshAttributeKind::Color => self.dirty |= MeshDirty::COLOR,
        }
    }

    pub fn is_attribute_dirty(&self, attribute: MeshAttributeKind) -> bool {
        match attribute {
            MeshAttributeKind::Position => self.dirty.contains(MeshDirty::POSITION),
            MeshAttributeKind::Normal => self.dirty.contains(MeshDirty::NORMAL),
            MeshAttributeKind::Tangent => self.dirty.contains(MeshDirty::TANGENT),
            MeshAttributeKind::TexCoord0 => self.dirty.contains(MeshDirty::TEXCOORD0),
            MeshAttributeKind::TexCoord1 => self.dirty.contains(MeshDirty::TEXCOORD1),
            MeshAttributeKind::Color => self.dirty.contains(MeshDirty::COLOR),
        }
    }

    pub fn layout(&self) -> MeshLayout {
        MeshLayout::from(self.attributes.iter().map(|a| a.kind()).collect::<Vec<_>>())
    }

    pub fn vertex_data(&self) -> (Vec<u8>, usize) {
        let count = self.vertex_count() as usize;
        let mut data = vec![];

        for index in 0..count {
            for attribute in &self.attributes {
                match attribute {
                    MeshAttribute::Position(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                    MeshAttribute::Normal(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                    MeshAttribute::TexCoord0(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                    MeshAttribute::TexCoord1(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                    MeshAttribute::Tangent(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                    MeshAttribute::Color(v) => {
                        data.extend_from_slice(bytemuck::bytes_of(&v[index]))
                    }
                }
            }
        }

        (data, count)
    }

    pub fn create_render_mesh(&mut self, device: &RenderDevice) -> RenderMesh {
        let (data, _) = self.vertex_data();

        let mut layout = vec![];
        let mut stride = 0;
        for attribute in &mut self.attributes {
            layout.push(attribute.kind());
            if self.read_write == ReadWrite::Disabled {
                attribute.clear();
            }
            stride += attribute.size();
        }

        let usage = match self.read_write {
            ReadWrite::Enabled => BufferUsages::COPY_DST | BufferUsages::MAP_WRITE,
            ReadWrite::Disabled => BufferUsages::empty(),
        };

        let vertex_buffer = VertexBuffer::new_from_data(device, &data, stride, Some(usage));

        let index_buffer = self.indices.as_mut().map(|indices| {
            let buffer = IndexBuffer::new(device, &indices, Some(usage));
            if self.read_write == ReadWrite::Disabled {
                indices.clear();
            }
            buffer
        });

        RenderMesh {
            layout: layout.into(),
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn update(&mut self, device: &RenderDevice, buffers: &mut RenderMesh) {
        let (data, _) = self.vertex_data();
        let stride = self.attributes.iter().fold(0, |sum, a| sum + a.size());

        buffers.vertex_buffer = VertexBuffer::new_from_data(device, &data, stride, None);

        if self.dirty.contains(MeshDirty::INDICES) {
            match (buffers.index_buffer.as_mut(), self.indices()) {
                (Some(index), Some(indices)) => {
                    index.update(device, indices);
                    self.dirty.remove(MeshDirty::INDICES);
                }
                _ => (),
            }
        }
    }
}

impl Asset for Mesh {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshLayout(Box<[MeshAttributeKind]>);

impl From<Vec<MeshAttributeKind>> for MeshLayout {
    fn from(attributes: Vec<MeshAttributeKind>) -> Self {
        Self(attributes.into_boxed_slice())
    }
}

impl From<&[MeshAttributeKind]> for MeshLayout {
    fn from(attributes: &[MeshAttributeKind]) -> Self {
        Self(attributes.to_vec().into_boxed_slice())
    }
}

impl<A: AsRef<[MeshAttributeKind]>> From<&A> for MeshLayout {
    fn from(attributes: &A) -> Self {
        Self(attributes.as_ref().to_vec().into_boxed_slice())
    }
}

impl std::ops::Deref for MeshLayout {
    type Target = [MeshAttributeKind];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MeshLayout {
    type Item = MeshAttributeKind;
    type IntoIter = std::vec::IntoIter<MeshAttributeKind>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a MeshLayout {
    type Item = &'a MeshAttributeKind;
    type IntoIter = std::slice::Iter<'a, MeshAttributeKind>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

pub struct RenderMesh {
    layout: MeshLayout,
    vertex_buffer: VertexBuffer,
    index_buffer: Option<IndexBuffer>,
}

impl RenderMesh {
    pub fn layout(&self) -> &MeshLayout {
        &self.layout
    }

    pub fn vertex_buffer(&self) -> &VertexBuffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> Option<&IndexBuffer> {
        self.index_buffer.as_ref()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_buffer.len()
    }

    pub fn index_count(&self) -> usize {
        self.index_buffer.as_ref().map_or(0, |i| i.len())
    }
}

impl From<&RenderMesh> for SubMesh {
    fn from(mesh: &RenderMesh) -> Self {
        SubMesh::new(0, mesh.vertex_count() as u64, 0, mesh.index_count() as u64)
    }
}

impl RenderAsset for RenderMesh {
    type Id = AssetId;
}

impl RenderAssetExtractor for Mesh {
    type Target = RenderMesh;
    type Arg = ReadRes<RenderDevice>;

    fn extract(
        _: &AssetId,
        mesh: &mut Self,
        device: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Target, ExtractError> {
        let buffers = mesh.create_render_mesh(device);
        Ok(buffers)
    }

    fn update(
        _: &AssetId,
        mesh: &mut Self,
        asset: &mut Self::Target,
        device: &mut ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        Ok(mesh.update(&device, asset))
    }

    fn usage(_: &AssetId, source: &Self) -> AssetUsage {
        match source.read_write {
            ReadWrite::Enabled => AssetUsage::Keep,
            ReadWrite::Disabled => AssetUsage::Discard,
        }
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Target>, _: &mut ArgItem<Self::Arg>) {
        assets.remove(id);
    }
}
