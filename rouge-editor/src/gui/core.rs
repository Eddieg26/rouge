use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ElementId(u64);

impl ElementId {
    pub fn root() -> Self {
        ElementId(0)
    }

    pub fn child(&self, id: &str) -> ElementId {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        id.hash(&mut hasher);
        let hash = hasher.finish();
        ElementId(hash)
    }
}

impl std::ops::Deref for ElementId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ElementId({})", self.0)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Position {
    Relative { x: i32, y: i32 },
    Absolute { x: i32, y: i32 },
}

#[derive(Debug, Copy, Clone)]
pub enum SizeMode {
    Fixed(u32),
    Percent(f32),
    Flex(u16),
}

impl From<u32> for SizeMode {
    fn from(value: u32) -> Self {
        SizeMode::Fixed(value)
    }
}

impl From<f32> for SizeMode {
    fn from(value: f32) -> Self {
        SizeMode::Percent(value)
    }
}

impl From<u16> for SizeMode {
    fn from(value: u16) -> Self {
        SizeMode::Flex(value)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Size {
    width: SizeMode,
    height: SizeMode,
}

impl<L: Into<SizeMode>, R: Into<SizeMode>> From<(L, R)> for Size {
    fn from(value: (L, R)) -> Self {
        Size {
            width: value.0.into(),
            height: value.1.into(),
        }
    }
}

impl Size {
    pub fn new(width: SizeMode, height: SizeMode) -> Self {
        Size { width, height }
    }

    pub fn width(&self) -> SizeMode {
        self.width
    }

    pub fn height(&self) -> SizeMode {
        self.height
    }
}
