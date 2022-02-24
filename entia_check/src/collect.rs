use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Collect<G, C, F>(G, C, PhantomData<F>);
#[derive(Debug, Default)]
pub struct Shrinker<S, F>(Vec<S>, usize, PhantomData<F>);

impl<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
    #[inline]
    pub fn new(generate: G, count: C) -> Self {
        Self(generate, count, PhantomData)
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrinker<S, F> {
    #[inline]
    pub fn new(shrinks: Vec<S>) -> Self {
        Self(shrinks, 0, PhantomData)
    }
}

impl<G: Clone, C: Clone, F> Clone for Collect<G, C, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<S: Clone, F> Clone for Shrinker<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1, PhantomData)
    }
}

impl<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>> Generate
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (count, _) = self.1.generate(state);
        let mut shrinks = Vec::with_capacity(count);
        let items = Iterator::map(0..count, |_| {
            let (item, state) = self.0.generate(state);
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
        self.0.iter().map(|shrink| shrink.generate()).collect()
    }

    fn shrink(&mut self) -> Option<Self> {
        // Try to remove irrelevant generators.
        if self.1 < self.0.len() {
            let mut shrinks = self.0.clone();
            shrinks.remove(self.1);
            self.1 += 1;
            return Some(Self::new(shrinks));
        }

        // Try to shrink each generator and succeed if any generator is shrunk.
        for i in 0..self.0.len() {
            if let Some(shrink) = self.0[i].shrink() {
                let mut shrinks = self.0.clone();
                shrinks[i] = shrink;
                return Some(Self::new(shrinks));
            }
        }

        None
    }
}
