use crate::generator::{Generator, IntoGenerator, Shrinker, State};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Any<T>(T, usize);
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T>(pub T, pub f64);

macro_rules! collection {
    ($t:ty, $i:ident, [$($a:tt)?], [$($n:ident)?]) => {
        impl<T: Generator $(, const $n: usize)?> From<$t> for Any<$t> {
            fn from(generators: $t) -> Self {
                Self(generators, 0)
            }
        }

        impl<T: Generator $(, const $n: usize)?> Generator for Any<$t> {
            type Item = Option<T::Item>;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Some($i(self.0.as_mut(), &mut self.1, state)?.generate(state))
            }
        }

        impl<T: Shrinker $(, const $n: usize)?> Shrinker for Any<$t> {
            type Item = T::Item;
            type Generator = T::Generator;
            #[inline]
            fn shrink(&mut self) -> Option<T::Generator> {
                (&mut self.0).into_iter().nth(self.1)?$(.$a)?.shrink()
            }
        }
    };
}

macro_rules! tuple {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> From<($t, $($ts,)*)> for Any<($t, $($ts,)*)> {
            fn from(generators: ($t, $($ts,)*)) -> Self {
                Self(generators, 0)
            }
        }

        impl<$t: IntoGenerator, $($ts: IntoGenerator<Item = $t::Item>,)*> IntoGenerator for Any<($t, $($ts,)*)> {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = Any<($t::Generator, $($ts::Generator,)*)>;
            #[inline]
            fn generator() -> Self::Generator {
                Any(($t::generator(), $($ts::generator(),)*), 0)
            }
        }

        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<($t, $($ts,)*)> {
            type Item = $t::Item;
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let ($p, $($ps,)*) = &mut self.0;
                let count = entia_macro::count!($p $(,$ps)*);
                let mut _index = state.random.u8(..count);
                self.1 = _index as usize;
                if _index == 0 { return $p.generate(state); }
                $(_index -= 1; if _index == 0 { return $ps.generate(state); })*
                unreachable!();
            }
        }

        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
            type Item = $t::Item;
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let ($p, $($ps,)*) = &mut self.0;
                let total = $p.1 $(+ $ps.1)*;
                let mut _weight = state.random.f64() * total;
                let mut _index = 0;
                if _weight < $p.1 { self.1 = _index; return $p.0.generate(state); } else { _weight -= $p.1; }
                $(_index += 1; if _weight < $ps.1 { self.1 = _index; return $ps.0.generate(state); } else { _weight -= $ps.1; })*
                unreachable!();
            }
        }

        mod $t {
            use super::*;

            pub enum One<$t, $($ts,)*> {
                $t($t),
                $($ts($ts),)*
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for One<$t, $($ts,)*> {
                type Item = $t::Item;
                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    match self {
                        One::$t($p) => $p.generate(state),
                        $(One::$ts($ps) => $ps.generate(state),)*
                    }
                }
            }

            impl<$t: Shrinker, $($ts: Shrinker<Item = $t::Item>,)*> Shrinker for One<$t, $($ts,)*> {
                type Item = $t::Item;
                type Generator = One<$t::Generator, $($ts::Generator,)*>;
                fn shrink(&mut self) -> Option<Self::Generator> {
                    match self {
                        One::$t($p) => Some(One::$t($p.shrink()?)),
                        $(One::$ts($ps) => Some(One::$ts($ps.shrink()?)),)*
                    }
                }
            }

            impl<$t: Shrinker, $($ts: Shrinker<Item = $t::Item>,)*> Shrinker for Any<($t, $($ts,)*)> {
                type Item = $t::Item;
                type Generator = One<$t::Generator, $($ts::Generator,)*>;
                fn shrink(&mut self) -> Option<Self::Generator> {
                    let ($p, $($ps,)*) = &mut self.0;
                    let mut _index = self.1;
                    if _index == 0 { return Some(One::$t($p.shrink()?)); }
                    $(_index -= 1; if _index == 0 { return Some(One::$ts($ps.shrink()?)); })*
                    unreachable!();
                }
            }
        }
    };
}

collection!([T; N], indexed, [], [N]);
collection!([Weight<T>; N], weighted, [0], [N]);
collection!(Vec<T>, indexed, [], []);
collection!(Vec<Weight<T>>, weighted, [0], []);
entia_macro::recurse_16!(tuple);

fn indexed<'a, T>(
    items: &'a mut [T],
    index: &'a mut usize,
    state: &mut State,
) -> Option<&'a mut T> {
    *index = state.random.usize(..items.len());
    items.get_mut(*index)
}

fn weighted<'a, T>(
    items: &'a mut [Weight<T>],
    index: &'a mut usize,
    state: &mut State,
) -> Option<&'a mut T> {
    let total = items.iter().map(|weight| weight.1).sum::<f64>();
    let mut random = state.random.f64() * total;
    for (i, weight) in items.iter_mut().enumerate() {
        if random < weight.1 {
            *index = i;
            return Some(&mut weight.0);
        } else {
            random -= weight.1;
        }
    }
    None
}
