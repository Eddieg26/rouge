use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rect<T = f32> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl<T: Add<Output = T> + Copy> Rect<T> {
    pub fn left(&self) -> T {
        self.x
    }

    pub fn right(&self) -> T {
        self.x + self.width
    }

    pub fn top(&self) -> T {
        self.y
    }

    pub fn bottom(&self) -> T {
        self.y + self.height
    }
}

impl<T: Add<Output = T> + Copy> Add for Rect<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

impl<T: Sub<Output = T> + Copy> Sub for Rect<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul for Rect<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            width: self.width * rhs.width,
            height: self.height * rhs.height,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div for Rect<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            width: self.width / rhs.width,
            height: self.height / rhs.height,
        }
    }
}

impl<T: AddAssign + Copy> AddAssign for Rect<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.width += rhs.width;
        self.height += rhs.height;
    }
}

impl<T: SubAssign + Copy> SubAssign for Rect<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.width -= rhs.width;
        self.height -= rhs.height;
    }
}

impl<T: MulAssign + Copy> MulAssign for Rect<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.width *= rhs.width;
        self.height *= rhs.height;
    }
}

impl<T: DivAssign + Copy> DivAssign for Rect<T> {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.width /= rhs.width;
        self.height /= rhs.height;
    }
}
