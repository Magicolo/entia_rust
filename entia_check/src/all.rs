use crate::generator::{Generator, IntoGenerator, State};
use entia_core::utility::array;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct All<T>(T);

impl<G: IntoGenerator, const N: usize> IntoGenerator for [G; N] {
    type Item = [G::Item; N];
    type Generator = All<[G::Generator; N]>;
    #[inline]
    fn generator() -> Self::Generator {
        All(array(|_| G::generator()))
    }
}

impl<G: Generator, const N: usize> From<[G; N]> for All<[G; N]> {
    fn from(generators: [G; N]) -> Self {
        Self(generators)
    }
}

impl<G: Generator, const N: usize> Generator for All<[G; N]> {
    type Item = [G::Item; N];
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        array(|i| self.0[i].generate(state))
    }
}

impl<G: Generator> From<Vec<G>> for All<Vec<G>> {
    fn from(generators: Vec<G>) -> Self {
        Self(generators)
    }
}

impl<G: Generator> Generator for All<Vec<G>> {
    type Item = Vec<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let map = |generator: &mut G| generator.generate(state);
        self.0.iter_mut().map(map).collect()
    }
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: IntoGenerator,)*> From<($($t,)*)> for All<($($t,)*)> {
            fn from(generators: ($($t,)*)) -> Self {
                Self(generators)
            }
        }

        impl<$($t: IntoGenerator,)*> IntoGenerator for ($($t,)*) {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = All<($($t::Generator,)*)>;
            #[inline]
            fn generator() -> Self::Generator {
                All(($($t::generator(),)*))
            }
        }

        impl<$($t: Generator,)*> Generator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            #[inline]
            fn generate(&mut self, _state: &mut State) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.generate(_state),)*)
            }
        }

        impl<$($t: Generator,)*> Generator for All<($($t,)*)> {
            type Item = ($($t::Item,)*);
            #[inline]
            fn generate(&mut self, _state: &mut State) -> Self::Item {
                let ($($p,)*) = &mut self.0;
                ($($p.generate(_state),)*)
            }
        }
    };
}

entia_macro::recurse_16!(tuple);
