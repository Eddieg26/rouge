use crate::primitives::object::{Object2D, Object3D};

pub trait Draw2D: Object2D + 'static {}

pub trait Draw3D: Object3D + 'static {}

pub struct Camera2D {
    pub id: u64,
    pub position: glam::Vec2,
    pub rotation: f32,
    pub size: f32,
    pub near: f32,
    pub far: f32,
    pub depth: usize,
    pub clear_color: wgpu::Color,
}

impl Camera2D {
    pub fn new(
        id: u64,
        position: glam::Vec2,
        rotation: f32,
        size: f32,
        near: f32,
        far: f32,
        depth: usize,
        clear_color: wgpu::Color,
    ) -> Camera2D {
        Camera2D {
            id,
            position,
            rotation,
            size,
            near,
            far,
            depth,
            clear_color,
        }
    }

    pub fn orthographic(&self, aspect: f32) -> glam::Mat4 {
        glam::Mat4::orthographic_rh_gl(
            -self.size * aspect,
            self.size * aspect,
            -self.size,
            self.size,
            self.near,
            self.far,
        )
    }

    pub fn view(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(
            glam::Quat::from_rotation_z(self.rotation),
            glam::vec3(self.position.x, self.position.y, 0.0),
        )
    }
}

pub struct Camera3D {
    pub id: u64,
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub depth: usize,
    pub clear_color: wgpu::Color,
}

impl Camera3D {
    pub fn new(
        id: u64,
        position: glam::Vec3,
        rotation: glam::Quat,
        fov: f32,
        near: f32,
        far: f32,
        depth: usize,
        clear_color: wgpu::Color,
    ) -> Camera3D {
        Camera3D {
            id,
            position,
            rotation,
            fov,
            near,
            far,
            depth,
            clear_color,
        }
    }

    pub fn orthographic(&self, aspect: f32) -> glam::Mat4 {
        glam::Mat4::orthographic_rh_gl(
            -self.fov * aspect,
            self.fov * aspect,
            -self.fov,
            self.fov,
            self.near,
            self.far,
        )
    }

    pub fn perspective(&self, aspect: f32) -> glam::Mat4 {
        glam::Mat4::perspective_rh_gl(self.fov, aspect, self.near, self.far)
    }

    pub fn view(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(self.rotation, self.position)
    }
}
