use crate::generate::{Generate, State};

#[derive(Debug)]
pub struct Sample<'a, G> {
    generate: &'a G,
    state: State,
}

impl<'a, G> Sample<'a, G> {
    pub fn new(generate: &'a G, state: State) -> Self {
        Self { generate, state }
    }
}

impl<G: Generate> Iterator for Sample<'_, G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.state = self.state.next()?;
        Some(self.generate.generate(&mut self.state).0)
    }
}

impl<G: Generate> ExactSizeIterator for Sample<'_, G> {
    #[inline]
    fn len(&self) -> usize {
        self.state.len()
    }
}
