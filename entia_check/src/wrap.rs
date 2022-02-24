use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Wrap<G, T, B = fn(&mut State) -> T, A = fn(&mut State, T)>(G, B, A, PhantomData<T>);

impl<G: Clone, T, B: Clone, A: Clone> Clone for Wrap<G, T, B, A> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2.clone(), PhantomData)
    }
}

impl<G: Generate, T, B: FnMut() -> T + Clone, A: FnMut(T) + Clone> Wrap<G, T, B, A> {
    #[inline]
    pub fn generator(generate: G, before: B, after: A) -> Self {
        Self(generate, before, after, PhantomData)
    }
}

impl<S: Shrink, T, B: FnMut() -> T + Clone, A: FnMut(T) + Clone> Wrap<S, T, B, A> {
    #[inline]
    pub fn shrink(shrink: S, before: B, after: A) -> Self {
        Self(shrink, before, after, PhantomData)
    }
}

impl<G: Generate, T, B: Fn() -> T + Clone, A: Fn(T) + Clone> Generate for Wrap<G, T, B, A> {
    type Item = G::Item;
    type Shrink = Wrap<G::Shrink, T, B, A>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let value = self.1();
        let (item, shrink) = self.0.generate(state);
        self.2(value);
        (
            item,
            Wrap(shrink, self.1.clone(), self.2.clone(), PhantomData),
        )
    }
}

impl<S: Shrink, T, B: Fn() -> T + Clone, A: Fn(T) + Clone> Shrink for Wrap<S, T, B, A> {
    type Item = S::Item;

    fn generate(&self) -> Self::Item {
        let value = self.1();
        let item = self.0.generate();
        self.2(value);
        item
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(
            self.0.shrink()?,
            self.1.clone(),
            self.2.clone(),
            PhantomData,
        ))
    }
}
