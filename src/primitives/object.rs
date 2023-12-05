use super::{bounds::Bounds, rect::Rect};

pub trait Object2D {
    type Identifier: Eq + PartialEq + Copy;

    fn id(&self) -> Self::Identifier;
    fn bounds(&self) -> &Rect;
}

pub trait Object3D {
    type Identifier: Eq + PartialEq;

    fn id(&self) -> Self::Identifier;
    fn bounds(&self) -> &Bounds;
}
