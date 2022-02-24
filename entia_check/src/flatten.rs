use crate::generate::{Generate, State};

#[derive(Clone, Debug, Default)]
pub struct Flatten<G>(G);

impl<G: Generate<Item = impl Generate>> Flatten<G> {
    pub fn new(generate: G) -> Self {
        Self(generate)
    }
}

impl<G: Generate<Item = impl Generate>> Generate for Flatten<G> {
    type Item = <G::Item as Generate>::Item;
    type Shrink = <G::Item as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (item, _) = self.0.generate(state);
        item.generate(state)
    }
}
