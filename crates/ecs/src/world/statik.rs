use std::hash::Hash;

pub trait StaticRef: Hash + Eq {
    fn ref_equal(&self, other: &Self) -> bool;

    fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H);
}

impl StaticRef for str {
    fn ref_equal(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr() && self.len() == other.len()
    }

    fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        self.as_ptr().hash(state);
    }
}

impl StaticRef for [u8] {
    fn ref_equal(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr() && self.len() == other.len()
    }

    fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        self.as_ptr().hash(state);
    }
}

pub struct Static<T: ?Sized + 'static>(&'static T);

impl<T: ?Sized + 'static> Static<T> {
    pub const fn new(value: &'static T) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &'static T {
        self.0
    }
}

impl<T: ?Sized + 'static> std::ops::Deref for Static<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: ?Sized + 'static> Clone for Static<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + 'static> Copy for Static<T> {}

impl<T: ?Sized + StaticRef> PartialEq for Static<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.ref_equal(other.0)
    }
}

impl<T: ?Sized + StaticRef> Hash for Static<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.ref_hash(state);
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for Static<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: ?Sized + StaticRef> Eq for Static<T> {}
