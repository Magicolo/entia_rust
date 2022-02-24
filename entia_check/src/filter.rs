use crate::generate::{Generate, State};

#[derive(Clone, Debug, Default)]
pub struct Filter<G, F = fn(&<G as Generate>::Item) -> bool>(G, F, usize);

impl<G: Generate, F: Fn(&G::Item) -> bool + Clone> Filter<G, F> {
    #[inline]
    pub fn new(generate: G, filter: F, iterations: usize) -> Self {
        Self(generate, filter, iterations)
    }
}

impl<G: Generate, F: Fn(&G::Item) -> bool + Clone> Generate for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        for _ in 0..self.2 {
            let (item, shrink) = self.0.generate(state);
            if self.1(&item) {
                return (Some(item), Some(shrink));
            }
        }
        (None, None)
    }
}
