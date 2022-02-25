use crate::generate::{Generate, State};

#[derive(Clone, Debug, Default)]
pub struct Flatten<T: ?Sized>(pub T);

impl<G: Generate<Item = impl Generate> + ?Sized> Generate for Flatten<G> {
    type Item = <G::Item as Generate>::Item;
    type Shrink = <G::Item as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (item, _) = self.0.generate(state);
        item.generate(state)
    }
}
