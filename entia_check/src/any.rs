use crate::{
    generator::{Generate, State},
    recurse,
    shrink::Shrink,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<G>(G);
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T>(pub T, pub f64);

fn indexed<'a, T>(items: &'a [T], state: &mut State) -> Option<&'a T> {
    if items.len() == 0 {
        None
    } else {
        Some(&items[state.random.usize(0..items.len())])
    }
}

fn weighted<'a, T>(items: &'a [Weight<T>], state: &mut State) -> Option<&'a T> {
    let total = items.iter().map(|weight| weight.1).sum::<f64>();
    let mut random = state.random.f64() * total;
    for weight in items {
        if random < weight.1 {
            return Some(&weight.0);
        } else {
            random -= weight.1;
        }
    }
    None
}

macro_rules! collection {
    ($t:ty, $i:ident, [$($l:lifetime)?], [$($a:tt)?], [$($n:ident)?]) => {
        impl<$($l,)? T: Generate $(,const $n: usize)?> From<$(&$l)? $t> for Any<$(&$l)? $t> {
            fn from(generates: $(&$l)? $t) -> Self {
                Self(generates)
            }
        }

        impl<$($l,)? T: Generate $(,const $n: usize)?> Generate for Any<$(&$l)? $t> {
            type Item = Option<T::Item>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                match $i(self.0.as_ref(), state) {
                    Some(generate) => {
                        let (item, shrink) = generate.generate(state);
                        (Some(item), Some(shrink))
                    }
                    None => (None, None)
                }
            }
        }
    };
}

collection!([T], indexed, ['a], [], []);
collection!([Weight<T>], weighted, ['a], [0], []);
collection!([T; N], indexed, [], [], [N]);
collection!([Weight<T>; N], weighted, [], [0], [N]);
collection!([T; N], indexed, ['a], [], [N]);
collection!([Weight<T>; N], weighted, ['a], [0], [N]);
collection!(Vec<T>, indexed, [], [], []);
collection!(Vec<Weight<T>>, weighted, [], [0], []);
collection!(Vec<T>, indexed, ['a], [], []);
collection!(Vec<Weight<T>>, weighted, ['a], [0], []);

macro_rules! tuple {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> From<($t, $($ts,)*)> for Any<($t, $($ts,)*)> {
            fn from(generates: ($t, $($ts,)*)) -> Self {
                Self(generates)
            }
        }

        // impl<$t: IntoGenerate, $($ts: IntoGenerate<Item = $t::Item>,)*> IntoGenerate for ($t, $($ts,)*) {
        //     type Item = $t::Item;
        //     type Generate = Any<($t::Generate, $($ts::Generate,)*), $p::One<$t, $($ts,)*>>;
        //     fn generator(self) -> Self::Generate {
        //         let ($p, $($ps,)*) = self;
        //         Any::Generate(($p.generator(), $($ps.generator(),)*))
        //     }
        // }

        // impl<$t: FullGenerate, $($ts: FullGenerate<Item = $t::Item>,)*> FullGenerate for Any<($t, $($ts,)*), $p::One<$t, $($ts,)*>> {
        //     type Item = <Self::Generate as Generate>::Item;
        //     type Generate = Any<($t::Generate, $($ts::Generate,)*)>;
        //     fn generator() -> Self::Generate {
        //         Any(($t::generator(), $($ts::generator(),)*))
        //     }
        // }

        pub(crate) mod $p {
            use super::*;

            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
            pub enum One<$t, $($ts = $t,)*> {
                $t($t),
                $($ts($ts),)*
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for One<$t, $($ts,)*> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match self {
                        Self::$t(generate) => { let (item, shrink) = generate.generate(state); (item, One::$t(shrink)) },
                        $(Self::$ts(generate) => { let (item, shrink) = generate.generate(state); (item, One::$ts(shrink)) },)*
                    }
                }
            }

            impl<$t: Shrink, $($ts: Shrink<Item = $t::Item>,)*> Shrink for One<$t, $($ts,)*> {
                type Item = $t::Item;

                fn generate(&self) -> Self::Item {
                    match self {
                        One::$t(shrink) => shrink.generate(),
                        $(One::$ts(shrink) => shrink.generate(),)*
                    }
                }

                fn shrink(&mut self) -> Option<Self> {
                    match self {
                        Self::$t(shrink) => Some(Self::$t(shrink.shrink()?)),
                        $(Self::$ts(shrink) => Some(Self::$ts(shrink.shrink()?)),)*
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for Any<($t, $($ts,)*)> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let ($p, $($ps,)*) = &self.0;
                    let count = entia_macro::count!($p $(,$ps)*);
                    let mut _index = state.random.u8(..count);
                    if _index == 0 { let (item, shrink) = $p.generate(state); return (item, One::$t(shrink)); }
                    $(_index -= 1; if _index == 0 { let (item, shrink) = $ps.generate(state); return (item, One::$ts(shrink)); })*
                    unreachable!();
                }
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let ($p, $($ps,)*) = &self.0;
                    let total = $p.1 $(+ $ps.1)*;
                    let mut _weight = state.random.f64() * total;
                    let mut _index = 0;
                    if _weight < $p.1 { let (item, shrink) = $p.0.generate(state); return (item, One::$t(shrink)); } else { _weight -= $p.1; }
                    $(_index += 1; if _weight < $ps.1 { let (item, shrink) = $ps.0.generate(state); return (item, One::$ts(shrink)); } else { _weight -= $ps.1; })*
                    unreachable!();
                }
            }
        }
    };
}

recurse!(tuple);
