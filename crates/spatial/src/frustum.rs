use crate::{bounds::BoundingBox, sphere::Sphere};
use glam::{Vec3, Vec3A, Vec4};

#[derive(Debug, Clone, Copy)]
pub struct HalfSpace {
    pub plane: Vec4,
}

impl HalfSpace {
    /// Returns the unit normal vector of the bisecting plane that characterizes the `HalfSpace`.
    #[inline]
    pub fn normal(&self) -> Vec3A {
        Vec3A::from_vec4(self.plane)
    }
}

impl std::ops::Deref for HalfSpace {
    type Target = Vec4;
    fn deref(&self) -> &Self::Target {
        &self.plane
    }
}

impl std::ops::DerefMut for HalfSpace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.plane
    }
}

pub struct Frustum {
    pub planes: [HalfSpace; 6],
}

impl Frustum {
    pub fn from_view_projection(view_projection: glam::Mat4) -> Self {
        let mut frustum = Self::from_view_projection_no_far(view_projection);
        frustum.planes[5] = HalfSpace {
            plane: view_projection.row(2),
        };
        frustum
    }

    pub fn from_view_projection_with_far(
        view_projection: glam::Mat4,
        view_translation: Vec3,
        view_backward: Vec3,
        far: f32,
    ) -> Frustum {
        let mut frustum = Self::from_view_projection_no_far(view_projection);
        let far_center = view_translation - far * view_backward;
        frustum.planes[5] = HalfSpace {
            plane: view_backward.extend(-view_backward.dot(far_center)),
        };
        frustum
    }

    fn from_view_projection_no_far(view_projection: glam::Mat4) -> Self {
        let row3 = view_projection.row(3);
        let mut planes = [HalfSpace { plane: Vec4::ZERO }; 6];
        for (i, plane) in planes.iter_mut().enumerate().take(5) {
            let row = view_projection.row(i / 2);
            plane.plane = if (i & 1) == 0 && i != 4 {
                row3 + row
            } else {
                row3 - row
            }
        }

        Self { planes }
    }

    pub fn intersects_sphere(&self, sphere: &Sphere, include_far: bool) -> bool {
        let amount = if include_far { 6 } else { 5 };
        let sphere_center = sphere.position.extend(1.0);
        for plane in &self.planes[..amount] {
            if plane.dot(sphere_center) + sphere.radius <= 0.0 {
                return false;
            }
        }
        true
    }

    pub fn intersects_bounds(
        &self,
        bounds: &BoundingBox,
        include_near: bool,
        include_far: bool,
    ) -> bool {
        let bounds_center = bounds.center().extend(1.0);
        for (index, plane) in self.planes.iter().enumerate() {
            if index == 4 && !include_near {
                continue;
            }
            if index == 5 && !include_far {
                continue;
            }

            let relative_radius = bounds.relative_radius(plane.normal());
            if plane.dot(bounds_center) + relative_radius <= 0.0 {
                return false;
            }
        }

        true
    }
}
