use super::Point2D;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub position: glam::Vec2,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        position: glam::Vec2::ZERO,
        width: 0.0,
        height: 0.0,
    };

    pub const MAX: Rect = Rect {
        position: glam::Vec2::MIN,
        width: f32::MAX,
        height: f32::MAX,
    };

    pub fn new(left: f32, top: f32, width: f32, height: f32) -> Rect {
        Rect {
            position: glam::Vec2::new(left, top),
            width,
            height,
        }
    }

    pub fn left(&self) -> f32 {
        self.position.x
    }

    pub fn top(&self) -> f32 {
        self.position.y
    }

    pub fn bottom(&self) -> f32 {
        self.position.y + self.height
    }

    pub fn right(&self) -> f32 {
        self.position.x + self.width
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn size(&self) -> glam::Vec2 {
        glam::Vec2::new(self.width, self.height)
    }

    pub fn closest_point(&self, point: glam::Vec2) -> glam::Vec2 {
        let points = self.points();
        let mut closest_distance = f32::MAX;
        let mut return_point = points[0];

        for inner in points {
            let distance = inner.distance(point);
            if distance < closest_distance {
                closest_distance = distance;
                return_point = inner;
            }
        }

        return_point
    }

    pub fn points(&self) -> [Point2D; 4] {
        let top_left = glam::Vec2::new(self.left(), self.top());
        let top_right = glam::Vec2::new(self.left() + self.width, self.top());
        let bottom_right = glam::Vec2::new(self.left() + self.width, self.top() + self.height);
        let bottom_left = glam::Vec2::new(self.left(), self.top() + self.height);

        [top_left, top_right, bottom_right, bottom_left]
    }

    pub fn transform(&mut self, transform: glam::Mat4) {
        let points = self.points();
        let mut min = glam::Vec2::new(f32::MAX, f32::MAX);
        let mut max = glam::Vec2::new(f32::MIN, f32::MIN);

        for point in points.iter() {
            let transformed = transform.transform_point3(glam::Vec3::new(point.x, point.y, 0.0));
            min = glam::Vec2::new(min.x.min(transformed.x), min.y.min(transformed.y));
            max = glam::Vec2::new(max.x.max(transformed.x), max.y.max(transformed.y));
        }

        self.position = min;
        self.width = max.x - min.x;
        self.height = max.y - min.y;
    }
}
