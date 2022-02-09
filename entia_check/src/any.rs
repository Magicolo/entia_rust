use crate::{
    generator::{Generator, State},
    recurse,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Any<G, S = G> {
    Generate(G),
    Shrink(S),
}
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
    ($t:ty, $i:ident, [$($l:lifetime)?], [$($a:tt)?], [$($n:ident)?]) => {
        impl<$($l,)? T: Generator $(, const $n: usize)?> From<$(&$l)? $t> for Any<$(&$l)? $t, T> {
            fn from(generators: $(&$l)? $t) -> Self {
                Self::Generate(generators)
            }
        }

        impl<$($l,)? T: Generator $(, const $n: usize)?> Generator for Any<$(&$l)? $t, T> {
            type Item = Option<T::Item>;
            type State = Option<(T::State, usize)>;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                match self {
                    Any::Generate(generators) => {
                        if let Some((generator, index)) = $i(generators.as_ref(), state) {
                            let (item, state) = generator.generate(state);
                            (Some(item), Some((state, index)))
                        } else {
                            (None, None)
                        }
                    }
                    Any::Shrink(generator) => {let (item, state) = generator.generate(state); (Some(item), Some((state, 0))) },
                }
            }

            fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                let (state, index) = state.as_mut()?;
                Some(Any::Shrink(match self {
                    Any::Generate(generators) => generators[*index] $(.$a)? .shrink(state)?,
                    Any::Shrink(generator) => generator.shrink(state)?
                }))
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
        impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> From<($t, $($ts,)*)> for Any<($t, $($ts,)*), $p::One<$t, $($ts,)*>> {
            fn from(generators: ($t, $($ts,)*)) -> Self {
                Self::Generate(generators)
            }
        }

        // impl<$t: IntoGenerator, $($ts: IntoGenerator<Item = $t::Item>,)*> IntoGenerator for ($t, $($ts,)*) {
        //     type Item = $t::Item;
        //     type Generator = Any<($t::Generator, $($ts::Generator,)*), $p::One<$t, $($ts,)*>>;
        //     fn generator(self) -> Self::Generator {
        //         let ($p, $($ps,)*) = self;
        //         Any::Generate(($p.generator(), $($ps.generator(),)*))
        //     }
        // }

        // impl<$t: FullGenerator, $($ts: FullGenerator<Item = $t::Item>,)*> FullGenerator for Any<($t, $($ts,)*), $p::One<$t, $($ts,)*>> {
        //     type Item = <Self::Generator as Generator>::Item;
        //     type Generator = Any<($t::Generator, $($ts::Generator,)*)>;
        //     fn generator() -> Self::Generator {
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

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for One<$t, $($ts,)*> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    match self {
                        One::$t(generator) => { let (item, state) = generator.generate(state); (item, One::$t(state)) },
                        $(One::$ts(generator) => { let (item, state) = generator.generate(state); (item, One::$ts(state)) },)*
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    match (self, state) {
                        (One::$t(generator), One::$t(state)) => Some(One::$t(generator.shrink(state)?)),
                        $((One::$ts(generator), One::$ts(state)) => Some(One::$ts(generator.shrink(state)?)),)*
                        // The pattern is unreachable for '(T,)' since there is only one enum constructor.
                        #[allow(unreachable_patterns)]
                        _ => None,
                    }
                }
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<($t, $($ts,)*), One<$t, $($ts,)*>> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    match self {
                        Any::Generate(($p, $($ps,)*)) => {
                            let count = entia_macro::count!($p $(,$ps)*);
                            let mut _index = state.random.u8(..count);
                            if _index == 0 { let (item, state) = $p.generate(state); return (item, One::$t(state)); }
                            $(_index -= 1; if _index == 0 { let (item, state) = $ps.generate(state); return (item, One::$ts(state)); })*
                            unreachable!();
                        },
                        Any::Shrink(generator) => generator.generate(state),
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(Any::Shrink(match self {
                        Any::Generate(($p, $($ps,)*)) => match state {
                            One::$t(state) => One::$t($p.shrink(state)?),
                            $(One::$ts(state) => One::$ts($ps.shrink(state)?),)*
                        }
                        Any::Shrink(generator) => generator.shrink(state)?,
                    }))
                }
            }

            impl<$t: Generator, $($ts: Generator<Item = $t::Item>,)*> Generator for Any<(Weight<$t>, $(Weight<$ts>,)*), One<$t, $($ts,)*>> {
                type Item = $t::Item;
                type State = One<$t::State, $($ts::State,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    match self {
                        Any::Generate(($p, $($ps,)*)) => {
                            let total = $p.1 $(+ $ps.1)*;
                            let mut _weight = state.random.f64() * total;
                            let mut _index = 0;
                            if _weight < $p.1 { let (item, state) = $p.0.generate(state); return (item, One::$t(state)); } else { _weight -= $p.1; }
                            $(_index += 1; if _weight < $ps.1 { let (item, state) = $ps.0.generate(state); return (item, One::$ts(state)); } else { _weight -= $ps.1; })*
                            unreachable!();
                        }
                        Any::Shrink(generator) => generator.generate(state),
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(Any::Shrink(match self {
                        Any::Generate(($p, $($ps,)*)) => match state {
                            One::$t(state) => One::$t($p.0.shrink(state)?),
                            $(One::$ts(state) => One::$ts($ps.0.shrink(state)?),)*
                        }
                        Any::Shrink(generator) => generator.shrink(state)?,
                    }))
                }
            }
        }
    };
}

recurse!(tuple);
