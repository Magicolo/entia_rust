use crate::{
    generator::{or::Or, FullGenerator, Generator, State},
    recurse,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<T>(T);
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T>(pub T, pub f64);

fn indexed<'a, T>(items: &'a [T], state: &mut State) -> Option<(&'a T, usize)> {
    if items.len() == 0 {
        None
    } else {
        let index = state.random.usize(0..items.len());
        Some((&items[index], index))
    }
}

fn weighted<'a, T>(items: &'a [Weight<T>], state: &mut State) -> Option<(&'a T, usize)> {
    let total = items.iter().map(|weight| weight.1).sum::<f64>();
    let mut random = state.random.f64() * total;
    for (i, weight) in items.iter().enumerate() {
        if random < weight.1 {
            return Some((&weight.0, i));
        } else {
            random -= weight.1;
        }
    }
    None
}

macro_rules! collection {
    ($t:ty, $i:ident, [$($a:tt)?], [$($n:ident)?]) => {
        impl<T: Generator $(, const $n: usize)?> From<$t> for Any<$t> {
            fn from(generators: $t) -> Self {
                Self(generators)
            }
        }

        impl<T: Generator $(, const $n: usize)?> Generator for Any<$t> {
            type Item = Option<T::Item>;
            type State = Option<(T::State, usize)>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                if let Some((generator, index)) = $i(self.0.as_ref(), state) {
                    let (item, state) = generator.generate(state);
                    (Some(item), Some((state, index)))
                } else {
                    (None, None)
                }
            }

            fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                let (state, index) = state.as_mut()?;
                Some((&self.0).into_iter().nth(*index)?$(.$a)?.shrink(state))
            }
        }
    };
}

macro_rules! tuple {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> From<($t, $($ts,)*)> for Any<($t, $($ts,)*)> {
            fn from(generators: ($t, $($ts,)*)) -> Self {
                Self(generators)
            }
        }

        impl<$t: FullGenerator, $($ts: FullGenerator<Item = $t::Item>,)*> FullGenerator for Any<($t, $($ts,)*)> {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = Any<($t::Generator, $($ts::Generator,)*)>;
            fn generator() -> Self::Generator {
                Any(($t::generator(), $($ts::generator(),)*))
            }
        }

        mod $t {
            use super::*;

            #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
            pub enum One<$t, $($ts,)*> {
                $t($t),
                $($ts($ts),)*
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for One<$t, $($ts,)*> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    match self {
                        One::$t(generator) => { let (item, state) = generator.generate(state); (item, One::$t(state)) },
                        $(One::$ts(generator) => { let (item, state) = generator.generate(state); (item, One::$ts(state)) },)*
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    match (self, state) {
                        (One::$t(generator), One::$t(state)) => Some(One::$t(generator.shrink(state)?)),
                        $((One::$ts(generator), One::$ts(state)) => Some(One::$ts(generator.shrink(state)?)),)*
                        // The pattern is unreachable for '(T,)' since there is only one enum constructor.
                        #[allow(unreachable_patterns)]
                        _ => None,
                    }
                }
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<($t, $($ts,)*)> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;
                type Shrink = One<Or<$t, $t::Shrink>, $(Or<$ts, $ts::Shrink>,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let ($p, $($ps,)*) = &self.0;
                    let count = entia_macro::count!($p $(,$ps)*);
                    let mut _index = state.random.u8(..count);
                    if _index == 0 { let (item, state) = $p.generate(state); return (item, One::$t(state)); }
                    $(_index -= 1; if _index == 0 { let (item, state) = $ps.generate(state); return (item, One::$ts(state)); })*
                    unreachable!();
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    let ($p, $($ps,)*) = &self.0;
                    match state {
                        One::$t(state) => Some(One::$t(match $p.shrink(state) {
                            Some(generator) => Or::Right(generator),
                            None => Or::Left($p.clone()),
                        })),
                        $(One::$ts(state) => Some(One::$ts(match $ps.shrink(state) {
                            Some(generator) => Or::Right(generator),
                            None => Or::Left($ps.clone()),
                        })),)*
                    }
                }
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;
                type Shrink = One<Or<$t, $t::Shrink>, $(Or<$ts, $ts::Shrink>,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let ($p, $($ps,)*) = &self.0;
                    let total = $p.1 $(+ $ps.1)*;
                    let mut _weight = state.random.f64() * total;
                    let mut _index = 0;
                    if _weight < $p.1 { let (item, state) = $p.0.generate(state); return (item, One::$t(state)); } else { _weight -= $p.1; }
                    $(_index += 1; if _weight < $ps.1 { let (item, state) = $ps.0.generate(state); return (item, One::$ts(state)); } else { _weight -= $ps.1; })*
                    unreachable!();
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    let ($p, $($ps,)*) = &self.0;
                    match state {
                        One::$t(state) => Some(One::$t(match $p.0.shrink(state) {
                            Some(generator) => Or::Right(generator),
                            None => Or::Left($p.0.clone()),
                        })),
                        $(One::$ts(state) => Some(One::$ts(match $ps.0.shrink(state) {
                            Some(generator) => Or::Right(generator),
                            None => Or::Left($ps.0.clone()),
                        })),)*
                    }
                }
            }
        }
    };
}

collection!([T; N], indexed, [], [N]);
collection!([Weight<T>; N], weighted, [0], [N]);
collection!(Vec<T>, indexed, [], []);
collection!(Vec<Weight<T>>, weighted, [0], []);
recurse!(tuple);
