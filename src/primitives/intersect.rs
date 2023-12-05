use super::{bounds::Bounds, circle::Circle, plane::Plane, rect::Rect, sphere::Sphere};

pub trait Intersect2D {
    fn intersects_circle(&self, circle: &Circle) -> bool;
    fn intersects_rect(&self, rect: &Rect) -> bool;
}

pub trait Intersect3D {
    fn intersects_sphere(&self, sphere: &Sphere) -> bool;
    fn intersects_bounds(&self, bounds: &Bounds) -> bool;
    fn intersects_plane(&self, plane: &Plane) -> bool;
}

impl Intersect2D for Rect {
    fn intersects_circle(&self, circle: &Circle) -> bool {
        let closest_point = self.closest_point(circle.center);
        closest_point.distance(circle.center) < circle.size / 2.0
    }

    fn intersects_rect(&self, rect: &Rect) -> bool {
        self.left() < rect.right()
            && self.right() > rect.left()
            && self.top() < rect.bottom()
            && self.bottom() > rect.top()
    }
}

impl Intersect2D for Circle {
    fn intersects_circle(&self, circle: &Circle) -> bool {
        let distance = self.center.distance(circle.center);
        distance < self.size / 2.0 + circle.size / 2.0
    }

    fn intersects_rect(&self, rect: &Rect) -> bool {
        rect.intersects_circle(self)
    }
}

impl Intersect3D for Sphere {
    fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        let distance = self.center.distance(sphere.center);
        distance < self.size / 2.0 + sphere.size / 2.0
    }

    fn intersects_bounds(&self, bounds: &Bounds) -> bool {
        let closest_point = bounds.closest_point(self.center);
        let distance = closest_point.distance(self.center);
        distance < self.size / 2.0
    }

    fn intersects_plane(&self, plane: &Plane) -> bool {
        let distance = plane.normal.dot(self.center) + plane.distance;
        distance.abs() < self.size / 2.0
    }
}

impl Intersect3D for Bounds {
    fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        sphere.intersects_bounds(self)
    }

    fn intersects_bounds(&self, bounds: &Bounds) -> bool {
        let min1 = self.center - self.size;
        let max1 = self.center + self.size;
        let min2 = bounds.center - bounds.size;
        let max2 = bounds.center + bounds.size;

        let x_overlap = (max1.x >= min2.x) && (max2.x >= min1.x);
        let y_overlap = (max1.y >= min2.y) && (max2.x >= min1.y);
        let z_overlap = (max1.z >= min2.z) && (max2.z >= min1.z);

        x_overlap && y_overlap && z_overlap
    }

    fn intersects_plane(&self, plane: &Plane) -> bool {
        let half_extents = self.size / 2.0;

        let closest_point = glam::Vec3::new(
            plane.normal.x.signum() * half_extents.x,
            plane.normal.y.signum() * half_extents.y,
            plane.normal.z.signum() * half_extents.z,
        );

        let signed_distance = plane.normal.dot(closest_point);

        signed_distance.abs() <= plane.distance
    }
}

impl Intersect3D for Plane {
    fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        sphere.intersects_plane(self)
    }

    fn intersects_bounds(&self, bounds: &Bounds) -> bool {
        bounds.intersects_plane(self)
    }

    fn intersects_plane(&self, plane: &Plane) -> bool {
        let dot_product = self.normal.dot(plane.normal);

        if 1.0 - dot_product.abs() < std::f32::EPSILON {
            (plane.distance - self.distance).abs() < std::f32::EPSILON
        } else {
            true
        }
    }
}
