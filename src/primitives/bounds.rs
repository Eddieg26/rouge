use super::Point;

pub struct Bounds {
    pub center: glam::Vec3,
    pub size: glam::Vec3,
}

impl Bounds {
    pub const MAX: Bounds = Bounds {
        center: glam::Vec3::ZERO,
        size: glam::Vec3::MAX,
    };

    pub const ZERO: Bounds = Bounds {
        center: glam::Vec3::ZERO,
        size: glam::Vec3::ZERO,
    };

    pub fn new(center: glam::Vec3, size: glam::Vec3) -> Bounds {
        Bounds { center, size }
    }

    pub fn from_points(points: &[Point]) -> Bounds {
        let mut min = glam::Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = glam::Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);

            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }

        let center = (min + max) / 2.0;
        let size = max - center;

        Bounds { center, size }
    }

    pub fn points(&self) -> [Point; 8] {
        let min = self.center - self.size;
        let max = self.center + self.size;

        [
            Point::new(min.x, min.y, min.z),
            Point::new(min.x, min.y, max.z),
            Point::new(min.x, max.y, min.z),
            Point::new(min.x, max.y, max.z),
            Point::new(max.x, min.y, min.z),
            Point::new(max.x, min.y, max.z),
            Point::new(max.x, max.y, min.z),
            Point::new(max.x, max.y, max.z),
        ]
    }

    pub fn closest_point(&self, point: Point) -> Point {
        let min = self.center - self.size;
        let max = self.center + self.size;

        let mut x = point.x;
        let mut y = point.y;
        let mut z = point.z;

        if point.x < min.x {
            x = min.x;
        } else if point.x > max.x {
            x = max.x;
        }

        if point.y < min.y {
            y = min.y;
        } else if point.y > max.y {
            y = max.y;
        }

        if point.z < min.z {
            z = min.z;
        } else if point.z > max.z {
            z = max.z;
        }

        Point::new(x, y, z)
    }

    pub fn transform(&mut self, transform: glam::Mat4) {
        let points = self.points();
        let mut min = glam::Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = glam::Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for point in points.iter() {
            let transformed = transform.transform_point3(*point);
            min.x = min.x.min(transformed.x);
            min.y = min.y.min(transformed.y);
            min.z = min.z.min(transformed.z);

            max.x = max.x.max(transformed.x);
            max.y = max.y.max(transformed.y);
            max.z = max.z.max(transformed.z);
        }

        self.center = (min + max) / 2.0;
        self.size = max - self.center;
    }
}
