use crate::{
    array, collect,
    generator::{FullGenerate, Generate, IntoGenerate, State},
    recurse,
    shrink::Shrink,
};
use entia_core::Unzip;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct All<G>(pub G);

impl<G: FullGenerate, const N: usize> FullGenerate for [G; N] {
    type Item = [G::Item; N];
    type Generate = All<[G::Generate; N]>;
    fn generator() -> Self::Generate {
        All::from([(); N].map(|_| G::generator()))
    }
}

impl<G: Generate, const N: usize> From<[G; N]> for All<[G; N]> {
    fn from(generates: [G; N]) -> Self {
        Self(generates)
    }
}

impl<G: Generate, const N: usize> Generate for All<[G; N]> {
    type Item = [G::Item; N];
    type Shrink = array::Shrinker<G::Shrink, N>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let mut index = 0;
        let (items, shrinks) = [(); N]
            .map(|_| {
                let pair = self.0[index].generate(state);
                index += 1;
                pair
            })
            .unzip();
        (items, array::Shrinker(shrinks))
    }
}

impl<G: Generate> From<Vec<G>> for All<Vec<G>> {
    fn from(generates: Vec<G>) -> Self {
        Self(generates)
    }
}

impl<G: Generate> Generate for All<Vec<G>> {
    type Item = Vec<G::Item>;
    type Shrink = collect::Shrinker<G::Shrink, Vec<G::Item>>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (items, shrinks) = self
            .0
            .iter()
            .map(|generate| generate.generate(state))
            .unzip();
        (items, collect::Shrinker::new(shrinks))
    }
}

macro_rules! tuple {
    () => {
        impl FullGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator() -> Self::Generate { () }
        }

        impl IntoGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator(self) -> Self::Generate { self }
        }

        impl Generate for () {
            type Item = ();
            type Shrink = ();
            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) { ((), ()) }
        }

        impl Shrink for () {
            type Item = ();
            fn generate(&self) -> Self::Item { () }
            fn shrink(&mut self) -> Option<Self> { None }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t: FullGenerate,)*> FullGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator() -> Self::Generate {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: IntoGenerate,)*> IntoGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator(self) -> Self::Generate {
                let ($($p,)*) = self;
                ($($p.generator(),)*)
            }
        }

        impl<$($t: Generate,)*> Generate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) {
                let ($($p,)*) = self;
                $(let $p = $p.generate(_state);)*
                (($($p.0,)*), ($($p.1,)*))
            }
        }

        impl<$($t: Shrink,)*> Shrink for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn generate(&self) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.generate(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                let ($($p,)*) = self;
                let mut shrunk = false;
                let ($($p,)*) = ($(
                    if shrunk { $p.clone() }
                    else {
                        match $p.shrink() {
                            Some(shrink) => { shrunk = true; shrink },
                            None => $p.clone(),
                        }
                    },
                )*);
                if shrunk { Some(($($p,)*)) } else { None }
            }
        }
    };
}

recurse!(tuple);
