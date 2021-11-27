use crate::generator::{Generator, IntoGenerator, State};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Any<T>(pub T);
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct Weight<T>(pub T, pub f64);

impl<G: Generator, const N: usize> Generator for Any<[G; N]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(indexed(self.0.as_mut(), state)?.generate(state))
    }
}

impl<G: Generator, const N: usize> Generator for Any<[Weight<G>; N]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(weighted(self.0.as_mut(), state)?.generate(state))
    }
}

impl<G: Generator, const N: usize> Generator for Any<&'_ mut [G; N]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(indexed(self.0.as_mut(), state)?.generate(state))
    }
}

impl<G: Generator, const N: usize> Generator for Any<&'_ mut [Weight<G>; N]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(weighted(self.0.as_mut(), state)?.generate(state))
    }
}

impl<'a, G: Generator> Generator for Any<&'_ mut [G]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(indexed(self.0.as_mut(), state)?.generate(state))
    }
}

impl<'a, G: Generator> Generator for Any<&'_ mut [Weight<G>]> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(weighted(self.0.as_mut(), state)?.generate(state))
    }
}

impl<G: Generator> Generator for Any<Vec<G>> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(indexed(self.0.as_mut(), state)?.generate(state))
    }
}

impl<G: Generator> Generator for Any<Vec<Weight<G>>> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(weighted(self.0.as_mut(), state)?.generate(state))
    }
}

macro_rules! any {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$t: IntoGenerator, $($ts: IntoGenerator<Item = $t::Item>,)*> IntoGenerator for Any<($t, $($ts,)*)> {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = Any<($t::Generator, $($ts::Generator,)*)>;
            #[inline]
            fn generator() -> Self::Generator {
                Any(($t::generator(), $($ts::generator(),)*))
            }
        }

        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<($t, $($ts,)*)> {
            type Item = $t::Item;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let ($p, $($ps,)*) = &mut self.0;
                let count = entia_macro::count!($p $(,$ps)*);
                let mut _index = state.random.u8(..count);
                $(if _index == 0 { return $ps.generate(state); } else { _index -= 1; })*
                $p.generate(state)
            }
        }

        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
            type Item = $t::Item;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let ($p, $($ps,)*) = &mut self.0;
                let total = $p.1 $(+ $ps.1)*;
                let mut _weight = state.random.f64() * total;
                $(if _weight < $ps.1 { return $ps.0.generate(state); } else { _weight -= $ps.1; })*
                $p.0.generate(state)
            }
        }
    };
}

entia_macro::recurse_16!(any);

fn indexed<'a, T>(items: &'a mut [T], state: &mut State) -> Option<&'a mut T> {
    items.get_mut(state.random.usize(..items.len()))
}

fn weighted<'a, T>(items: &'a mut [Weight<T>], state: &mut State) -> Option<&'a mut T> {
    let total = items.iter().map(|weight| weight.1).sum::<f64>();
    let mut random = state.random.f64() * total;
    for weight in items.iter_mut() {
        if random < weight.1 {
            return Some(&mut weight.0);
        } else {
            random -= weight.1;
        }
    }
    None
}
