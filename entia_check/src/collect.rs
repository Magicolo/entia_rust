use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Collect<I: ?Sized, C, F> {
    _marker: PhantomData<F>,
    count: C,
    inner: I,
}

#[derive(Debug, Default)]
pub struct Shrinker<I, F> {
    inner: Vec<I>,
    index: usize,
    _marker: PhantomData<F>,
}

impl<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
    #[inline]
    pub fn new(generate: G, count: C) -> Self {
        Self {
            inner: generate,
            count,
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrinker<S, F> {
    #[inline]
    pub fn new(shrinks: Vec<S>) -> Self {
        Self {
            inner: shrinks,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, C: Clone, F> Clone for Collect<I, C, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: self.count.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, F> Clone for Shrinker<I, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, C: Generate<Item = usize>, F: FromIterator<G::Item>> Generate
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (count, _) = self.count.generate(state);
        let mut shrinks = Vec::with_capacity(count);
        let items = Iterator::map(0..count, |_| {
            let (item, state) = self.inner.generate(state);
            shrinks.push(state);
            item
        })
        .collect();
        (items, Shrinker::new(shrinks))
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrink for Shrinker<S, F> {
    type Item = F;

    fn generate(&self) -> Self::Item {
        self.inner.iter().map(|shrink| shrink.generate()).collect()
    }

    fn shrink(&mut self) -> Option<Self> {
        // Try to remove irrelevant generators.
        if self.index < self.inner.len() {
            let mut shrinks = self.inner.clone();
            shrinks.remove(self.index);
            self.index += 1;
            return Some(Self::new(shrinks));
        }

        // Try to shrink each generator and succeed if any generator is shrunk.
        for i in 0..self.inner.len() {
            if let Some(shrink) = self.inner[i].shrink() {
                let mut shrinks = self.inner.clone();
                shrinks[i] = shrink;
                return Some(Self::new(shrinks));
            }
        }

        None
    }
}
