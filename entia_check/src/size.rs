use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Default)]
pub struct Size<G>(G);

impl<G> Size<G> {
    #[inline]
    pub const fn new(generate: G) -> Self {
        Self(generate)
    }
}

impl<G> From<G> for Size<G> {
    #[inline]
    fn from(value: G) -> Self {
        Self::new(value)
    }
}

impl<T> Deref for Size<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Size<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
