use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Wrap<I: ?Sized, T: ?Sized, B = fn(&mut State) -> T, A = fn(&mut State, T)> {
    _marker: PhantomData<T>,
    before: B,
    after: A,
    inner: I,
}

impl<I: Clone, T, B: Clone, A: Clone> Clone for Wrap<I, T, B, A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            before: self.before.clone(),
            after: self.after.clone(),
            _marker: PhantomData,
        }
    }
}

impl<G: Generate, T, B: Fn() -> T, A: Fn(T)> Wrap<G, T, B, A> {
    #[inline]
    pub fn generator(generate: G, before: B, after: A) -> Self {
        Self {
            inner: generate,
            before,
            after,
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, T, B: Fn() -> T, A: Fn(T)> Wrap<S, T, B, A> {
    #[inline]
    pub fn shrinker(shrink: S, before: B, after: A) -> Self {
        Self {
            inner: shrink,
            before,
            after,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, T, B: Fn() -> T + Clone, A: Fn(T) + Clone> Generate
    for Wrap<G, T, B, A>
{
    type Item = G::Item;
    type Shrink = Wrap<G::Shrink, T, B, A>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let value = (self.before)();
        let (item, shrink) = self.inner.generate(state);
        (self.after)(value);
        (
            item,
            Wrap {
                inner: shrink,
                before: self.before.clone(),
                after: self.after.clone(),
                _marker: PhantomData,
            },
        )
    }
}

impl<S: Shrink, T, B: Fn() -> T + Clone, A: Fn(T) + Clone> Shrink for Wrap<S, T, B, A> {
    type Item = S::Item;

    fn generate(&self) -> Self::Item {
        let value = (self.before)();
        let item = self.inner.generate();
        (self.after)(value);
        item
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: self.inner.shrink()?,
            before: self.before.clone(),
            after: self.after.clone(),
            _marker: PhantomData,
        })
    }
}
