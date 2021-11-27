use crate::generator::{Generator, IntoGenerator, State};
use entia_core::utility::array;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct All<T>(pub T);

impl<G: IntoGenerator, const N: usize> IntoGenerator for [G; N] {
    type Item = [G::Item; N];
    type Generator = All<[G::Generator; N]>;
    #[inline]
    fn generator() -> Self::Generator {
        All(array(|_| G::generator()))
    }
}

impl<G: Generator, const N: usize> Generator for All<[G; N]> {
    type Item = [G::Item; N];
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        array(|i| self.0[i].generate(state))
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

impl<G: Generator> Generator for All<Box<[G]>> {
    type Item = Box<[G::Item]>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let map = |generator: &mut G| generator.generate(state);
        self.0.iter_mut().map(map).collect()
    }
}

macro_rules! all {
    ($($p:ident, $t:ident),*) => {
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

entia_macro::recurse_16!(all);
