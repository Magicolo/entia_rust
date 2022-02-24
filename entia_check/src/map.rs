use std::marker::PhantomData;

use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default)]
pub struct Map<G, T, F = fn(<G as Generate>::Item) -> T>(G, F, PhantomData<T>);

impl<G: Clone, T, F: Clone> Clone for Map<G, T, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> T + Clone> Map<G, T, F> {
    #[inline]
    pub fn generator(generate: G, map: F) -> Self {
        Self(generate, map, PhantomData)
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T + Clone> Map<S, T, F> {
    #[inline]
    pub fn shrink(shrink: S, map: F) -> Self {
        Self(shrink, map, PhantomData)
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> T + Clone> Generate for Map<G, T, F> {
    type Item = T;
    type Shrink = Map<G::Shrink, T, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (item, shrink) = self.0.generate(state);
        (self.1(item), Map::shrink(shrink, self.1.clone()))
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T + Clone> Shrink for Map<S, T, F> {
    type Item = T;

    fn generate(&self) -> Self::Item {
        self.1(self.0.generate())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self::shrink(self.0.shrink()?, self.1.clone()))
    }
}
