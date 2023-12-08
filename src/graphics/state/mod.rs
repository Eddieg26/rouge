use self::draw::{Camera2D, Camera3D, Draw2D, Draw3D};
use crate::{
    ecs::State,
    primitives::{bounds::Bounds, rect::Rect},
    tree::{OctTree, QuadTree},
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub mod draw;

pub struct RenderState {
    draw_2d: HashMap<TypeId, Box<dyn Any>>,
    draw_3d: HashMap<TypeId, Box<dyn Any>>,
    cameras_2d: Vec<Camera2D>,
    cameras_3d: Vec<Camera3D>,
}

impl RenderState {
    pub fn new() -> RenderState {
        RenderState {
            draw_2d: HashMap::new(),
            draw_3d: HashMap::new(),
            cameras_2d: Vec::new(),
            cameras_3d: Vec::new(),
        }
    }

    pub fn draw_2d<T: Draw2D>(&mut self, draw: T) {
        self.draw_2d
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(QuadTree::<T>::new(
                Rect::new(0.0, 0.0, 10000.0, 10000.0),
                5,
                30,
            )))
            .downcast_mut::<QuadTree<T>>()
            .unwrap()
            .insert(draw);
    }

    pub fn draw_3d<T: Draw3D>(&mut self, draw: T) {
        self.draw_3d
            .entry(TypeId::of::<T>())
            .or_insert(Box::<OctTree<T>>::new(OctTree::new(
                Bounds::new(glam::Vec3::ZERO, glam::vec3(10000.0, 10000.0, 10000.0)),
                5,
                30,
            )))
            .downcast_mut::<OctTree<T>>()
            .unwrap()
            .insert(draw);
    }

    pub fn render_2d(&mut self, camera: Camera2D) {
        self.cameras_2d.push(camera);
    }

    pub fn render_3d(&mut self, camera: Camera3D) {
        self.cameras_3d.push(camera);
    }
}

impl State for RenderState {
    fn reset(&mut self) {
        self.draw_2d.clear();
        self.draw_3d.clear();
        self.cameras_2d.clear();
        self.cameras_3d.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
