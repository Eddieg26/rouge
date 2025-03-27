use std::collections::HashMap;

use crate::{Color, Viewport};
use ecs::{
    Component, Entity, IndexMap, ResMut,
    derive::{Component, Resource},
    world::query::Query,
};
use spatial::{Rect, Transform};

use super::TextureView;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct CameraFlags: u8 {
        const MSAA = 0b00000001;
        const HDR = 0b00000010;
        const COLOR = 0b00000100;
        const DEPTH = 0b00001000;
    }
}

impl CameraFlags {
    pub fn new() -> Self {
        Self::COLOR | Self::DEPTH
    }
}

impl Default for CameraFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Component)]
pub struct Camera {
    pub order: i32,
    pub viewport: Option<Viewport>,
    pub clear_color: Option<Color>,
    pub flags: CameraFlags,
    pub enabled: bool,
}

pub trait Projection: Component + Send + Sync + 'static {
    fn projection(&self) -> glam::Mat4;
    fn update(&mut self, width: f32, height: f32);
    fn far(&self) -> f32;
}

#[derive(Component)]
pub struct Perspective {
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Projection for Perspective {
    fn projection(&self) -> glam::Mat4 {
        glam::Mat4::perspective_infinite_rh(self.fov, self.aspect, self.near)
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }

    fn far(&self) -> f32 {
        self.far
    }
}

#[derive(Clone, Copy, Component)]
pub struct Orthographic {
    pub rect: Rect,
    pub near: f32,
    pub far: f32,
}

impl Projection for Orthographic {
    fn projection(&self) -> glam::Mat4 {
        glam::Mat4::orthographic_rh(
            self.rect.left(),
            self.rect.right(),
            self.rect.bottom(),
            self.rect.top(),
            self.near,
            self.far,
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        self.rect.width = width;
        self.rect.height = height;
    }

    fn far(&self) -> f32 {
        self.far
    }
}

pub struct RenderCamera {
    camera: Camera,
    pub projection: glam::Mat4,
    pub view: glam::Mat4,
    pub world: glam::Mat4,
}

impl std::ops::Deref for RenderCamera {
    type Target = Camera;

    fn deref(&self) -> &Self::Target {
        &self.camera
    }
}

impl std::ops::DerefMut for RenderCamera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.camera
    }
}

#[derive(Resource)]
pub struct Cameras(IndexMap<Entity, RenderCamera>);

impl Cameras {
    pub fn new() -> Self {
        Self(IndexMap::default())
    }

    pub fn insert(&mut self, entity: Entity, camera: RenderCamera) {
        self.0.insert(entity, camera);
    }

    pub fn remove(&mut self, entity: &Entity) {
        self.0.shift_remove(entity);
    }

    pub fn get(&self, entity: &Entity) -> Option<&RenderCamera> {
        self.0.get(entity)
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<&mut RenderCamera> {
        self.0.get_mut(entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Entity, &RenderCamera)> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Entity, &mut RenderCamera)> {
        self.0.iter_mut()
    }

    pub fn sort(&mut self) {
        self.0.sort_by(|_, a, _, b| a.order.cmp(&b.order));
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

pub(crate) fn extract_cameras<P: Projection>(
    mut cameras: ResMut<Cameras>,
    query: Query<(Entity, &Camera, &P, &Transform)>,
) {
    cameras.clear();

    for (entity, camera, projection, transform) in query {
        let render_camera = RenderCamera {
            camera: camera.clone(),
            projection: projection.projection(),
            view: transform.local_to_world.inverse(),
            world: transform.local_to_world,
        };

        cameras.insert(entity, render_camera);
    }
}

pub struct RenderTargets {
    views: HashMap<Entity, TextureView>,
}
