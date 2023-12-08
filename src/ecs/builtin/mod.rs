use crate::ecs::{
    world::query::{Copied, Query},
    EntityId, World,
};

use super::Component;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Parent(pub EntityId);
impl Component for Parent {}

pub struct Children {
    pub children: Vec<EntityId>,
}

impl Children {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl Component for Children {}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Transform {
    pub fn new(position: glam::Vec3, rotation: glam::Quat, scale: glam::Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn matrix(&self, world: &World, parent: Option<Parent>) -> glam::Mat4 {
        let model =
            glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);

        if let Some(parent) = parent {
            if let Some((transform, parent)) = Query::<(
                Option<Copied<Transform>>,
                Option<Copied<Parent>>,
            )>::entity(world, parent.0)
            .next()
            {
                if let Some(transform) = transform {
                    return transform.matrix(world, parent) * model;
                }
            }
        }

        model
    }
}

impl Component for Transform {}
