#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb(r: f64, g: f64, b: f64) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self::new(r, g, b, a)
    }

    pub fn white() -> Self {
        Self::from_rgb(1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::from_rgb(0.0, 0.0, 0.0)
    }

    pub fn red() -> Self {
        Self::from_rgb(1.0, 0.0, 0.0)
    }

    pub fn green() -> Self {
        Self::from_rgb(0.0, 1.0, 0.0)
    }

    pub fn blue() -> Self {
        Self::from_rgb(0.0, 0.0, 1.0)
    }

    pub fn yellow() -> Self {
        Self::from_rgb(1.0, 1.0, 0.0)
    }

    pub fn magenta() -> Self {
        Self::from_rgb(1.0, 0.0, 1.0)
    }

    pub fn cyan() -> Self {
        Self::from_rgb(0.0, 1.0, 1.0)
    }

    pub fn transparent() -> Self {
        Self::from_rgba(0.0, 0.0, 0.0, 0.0)
    }

    pub fn hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let mut chars = hex.chars();
        let r = chars.next().unwrap().to_digit(16).unwrap() as f64 / 15.0;
        let g = chars.next().unwrap().to_digit(16).unwrap() as f64 / 15.0;
        let b = chars.next().unwrap().to_digit(16).unwrap() as f64 / 15.0;
        let a = if let Some(a) = chars.next() {
            a.to_digit(16).unwrap() as f64 / 15.0
        } else {
            1.0
        };
        Self::from_rgba(r, g, b, a)
    }

    pub fn to_hex(&self) -> String {
        let r = (self.r * 15.0).round() as u8;
        let g = (self.g * 15.0).round() as u8;
        let b = (self.b * 15.0).round() as u8;
        let a = (self.a * 15.0).round() as u8;
        format!("#{:x}{:x}{:x}{:x}", r, g, b, a)
    }
}

impl From<[f64; 3]> for Color {
    fn from([r, g, b]: [f64; 3]) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl From<[f64; 4]> for Color {
    fn from([r, g, b, a]: [f64; 4]) -> Self {
        Self::from_rgba(r, g, b, a)
    }
}

impl From<(f64, f64, f64)> for Color {
    fn from((r, g, b): (f64, f64, f64)) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl From<(f64, f64, f64, f64)> for Color {
    fn from((r, g, b, a): (f64, f64, f64, f64)) -> Self {
        Self::from_rgba(r, g, b, a)
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Self::from_rgb(r as f64, g as f64, b as f64)
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self::from_rgba(r as f64, g as f64, b as f64, a as f64)
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Self::from_rgb(r as f64, g as f64, b as f64)
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self::from_rgba(r as f64, g as f64, b as f64, a as f64)
    }
}

impl From<[u8; 3]> for Color {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::from_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self::from_rgba(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            a as f64 / 255.0,
        )
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::from_rgba(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            a as f64 / 255.0,
        )
    }
}

impl Into<[f64; 4]> for Color {
    fn into(self) -> [f64; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Into<[f64; 3]> for Color {
    fn into(self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        [self.r as f32, self.g as f32, self.b as f32, self.a as f32]
    }
}

impl Into<[f32; 3]> for Color {
    fn into(self) -> [f32; 3] {
        [self.r as f32, self.g as f32, self.b as f32]
    }
}
