use std::marker::PhantomData;

use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default)]
pub struct FilterMap<G, T, F = fn(<G as Generate>::Item) -> Option<T>>(G, F, usize, PhantomData<T>);
#[derive(Debug, Default)]
pub struct Shrinker<S, T, F = fn(<S as Shrink>::Item) -> Option<T>>(Option<S>, F, PhantomData<T>);

impl<G: Clone, T, F: Clone> Clone for FilterMap<G, T, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2, PhantomData)
    }
}

impl<G: Clone, T, F: Clone> Clone for Shrinker<G, T, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> Option<T> + Clone> FilterMap<G, T, F> {
    #[inline]
    pub fn new(generate: G, map: F, iterations: usize) -> Self {
        Self(generate, map, iterations, PhantomData)
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, T, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, T, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        for _ in 0..self.2 {
            let (item, shrink) = self.0.generate(state);
            if let Some(item) = self.1(item) {
                return (
                    Some(item),
                    Shrinker(Some(shrink), self.1.clone(), PhantomData),
                );
            }
        }
        (None, Shrinker(None, self.1.clone(), PhantomData))
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, T, F> {
    type Item = Option<T>;

    fn generate(&self) -> Self::Item {
        self.1(self.0.generate()?)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.shrink()?, self.1.clone(), PhantomData))
    }
}
