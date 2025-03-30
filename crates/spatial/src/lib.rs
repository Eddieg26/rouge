pub mod aabb;
pub mod bounds;
pub mod circle;
pub mod frustum;
pub mod plugin;
pub mod point;
pub mod rect;
pub mod size;
pub mod sphere;
pub mod transform;

pub use aabb::*;
pub use bounds::*;
pub use circle::*;
pub use frustum::*;
pub use point::*;
pub use rect::*;
pub use size::*;
pub use sphere::*;
pub use transform::*;

pub trait Mat3Ext {
    fn to_scale_rotation_translation(&self) -> (glam::Vec2, f32, glam::Vec2);
}

impl Mat3Ext for glam::Mat3 {
    fn to_scale_rotation_translation(&self) -> (glam::Vec2, f32, glam::Vec2) {
        // Extract the translation from the third column
        let translation = glam::Vec2::new(self.x_axis.z, self.y_axis.z);

        // Extract the scale from the first two columns (X and Y axes)
        let scale_x = (self.x_axis.x * self.x_axis.x + self.y_axis.x * self.y_axis.x).sqrt();
        let scale_y = (self.y_axis.y * self.x_axis.y + self.y_axis.y * self.y_axis.y).sqrt();
        let scale = glam::Vec2::new(scale_x, scale_y);

        // Normalize the first column to get the rotation without scaling
        let rotation_matrix = glam::Mat3::from_cols_slice(&[
            self.x_axis.x / scale_x,
            self.x_axis.y / scale_y,
            0.0,
            self.y_axis.x / scale_x,
            self.y_axis.y / scale_y,
            0.0,
            0.0,
            0.0,
            1.0,
        ]);

        // Calculate the rotation angle from the normalized rotation matrix
        let rotation = rotation_matrix.y_axis.x.atan2(rotation_matrix.x_axis.x);

        (translation, rotation, scale)
    }
}

use glam::Vec4;

pub struct RangeFinder {
    view_forward_z: Vec4,
}

impl RangeFinder {
    pub fn new(view_forward_z: Vec4) -> Self {
        Self { view_forward_z }
    }

    pub fn distance(&self, point: Vec4) -> f32 {
        self.view_forward_z.dot(point)
    }
}
