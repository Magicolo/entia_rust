use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use entia_core::Unzip;

#[derive(Clone, Debug, Default)]
pub struct Array<T: ?Sized, const N: usize>(pub T);
#[derive(Clone, Debug)]
pub struct Shrinker<T, const N: usize>(pub [T; N]);

impl<G: Generate + ?Sized, const N: usize> Generate for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = Shrinker<G::Shrink, N>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (items, shrinks) = [(); N].map(|_| self.0.generate(state)).unzip();
        (items, Shrinker(shrinks))
    }
}

impl<S: Shrink, const N: usize> Shrink for Shrinker<S, N> {
    type Item = [S::Item; N];

    fn generate(&self) -> Self::Item {
        let mut index = 0;
        [(); N].map(|_| {
            let item = self.0[index].generate();
            index += 1;
            item
        })
    }

    fn shrink(&mut self) -> Option<Self> {
        let mut index = 0;
        let mut shrunk = false;
        let shrinks = [(); N].map(|_| {
            let shrink = if shrunk { None } else { self.0[index].shrink() };
            let shrink = match shrink {
                Some(shrink) => {
                    shrunk = true;
                    shrink
                }
                None => self.0[index].clone(),
            };
            index += 1;
            shrink
        });

        if shrunk {
            Some(Self(shrinks))
        } else {
            None
        }
    }
}
