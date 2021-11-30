use self::{
    flatten::Flatten,
    map::Map,
    or::Or,
    sample::Samples,
    shrinker::{IntoShrinker, Shrinker},
};
use crate::{any::Any, primitive::Full};
use entia_core::utility;
use fastrand::Rng;
use std::{collections::VecDeque, iter::FromIterator, marker::PhantomData, mem::take, ops::Range};

pub trait IntoGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn generator() -> Self::Generator;
}

pub trait Generator {
    type Item;
    fn generate(&mut self, state: &mut State) -> Self::Item;

    #[inline]
    fn adapt<F: FnMut(&mut State)>(self, adapt: F) -> With<(Self, F), Self::Item>
    where
        Self: Sized,
    {
        With::new((self, adapt), |state, (generator, adapt)| {
            adapt(state);
            Some(generator.generate(state))
        })
    }

    #[inline]
    fn map<T, F: FnMut(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map::new(self, map)
    }

    #[inline]
    fn filter<F: FnMut(&Self::Item) -> bool>(
        self,
        filter: F,
    ) -> With<(Self, F, usize), Option<Self::Item>>
    where
        Self: Sized,
    {
        With::new((self, filter, 256), |state, (generator, filter, count)| {
            let value = generator.generate(state);
            if filter(&value) {
                Some(Some(value))
            } else if *count == 0 {
                Some(None)
            } else {
                *count -= 1;
                None
            }
        })
    }

    #[inline]
    fn filter_map<T, F: FnMut(Self::Item) -> Option<T>>(
        self,
        map: F,
    ) -> With<(Self, F, usize), Option<T>>
    where
        Self: Sized,
    {
        With::new((self, map, 256), |state, (generator, map, count)| {
            let value = generator.generate(state);
            if let Some(value) = map(value) {
                Some(Some(value))
            } else if *count == 0 {
                Some(None)
            } else {
                *count -= 1;
                None
            }
        })
    }

    #[inline]
    fn bind<G: Generator, F: FnMut(Self::Item) -> G>(self, bind: F) -> Flatten<G, Map<Self, G, F>>
    where
        Self: Sized,
    {
        self.map(bind).flatten()
    }

    #[inline]
    fn flatten(self) -> Flatten<Self::Item, Self>
    where
        Self: Sized,
        Self::Item: Generator,
    {
        Flatten::new(self)
    }

    #[inline]
    fn array<const N: usize>(self) -> With<Self, [Self::Item; N]>
    where
        Self: Sized,
    {
        With::new(self, |state, generator| {
            Some(utility::array(|_| generator.generate(state)))
        })
    }

    #[inline]
    fn collect<F: FromIterator<Self::Item>>(self) -> With<(Self, Size<Range<usize>>), F>
    where
        Self: Sized,
    {
        self.collect_with(Size(0..256))
    }

    #[inline]
    fn collect_with<C: Generator<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> With<(Self, C), F>
    where
        Self: Sized,
    {
        With::new((self, count), |state, (generator, count)| {
            Some(Iterator::map(0..count.generate(state), |_| generator.generate(state)).collect())
        })
    }

    #[inline]
    fn sample(self, count: usize) -> Samples<Self>
    where
        Self: Sized,
    {
        Samples::new(self, count)
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub random: Rng,
    pub size: f64,
    pub depth: usize,
    pub iteration: usize,
    pub iterations: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct With<S, T, F = fn(&mut State, &mut S) -> Option<T>>(S, F, PhantomData<T>);
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Size<G>(pub G);

impl<S, T, F: FnMut(&mut State, &mut S) -> Option<T>> With<S, T, F> {
    #[inline]
    pub fn new(state: S, with: F) -> Self {
        Self(state, with, PhantomData)
    }
}

impl IntoGenerator for String {
    type Item = Self;
    type Generator = With<(Size<Full<char>>, Size<Range<usize>>), Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        char::generator().collect()
    }
}

impl<G: IntoGenerator> IntoGenerator for Vec<G> {
    type Item = Vec<G::Item>;
    type Generator = With<(G::Generator, Size<Range<usize>>), Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        G::generator().collect()
    }
}

impl<S, T, F: FnMut(&mut State, &mut S) -> Option<T>> Generator for With<S, T, F> {
    type Item = T;

    fn generate(&mut self, state: &mut State) -> Self::Item {
        loop {
            match self.1(state, &mut self.0) {
                Some(value) => return value,
                None => {}
            }
        }
    }
}

pub mod sample {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Samples<G> {
        generator: G,
        random: Rng,
        indices: VecDeque<usize>,
        index: usize,
        count: usize,
    }

    impl<G> Samples<G> {
        pub fn new(generator: G, count: usize) -> Self {
            Self {
                generator,
                random: Rng::new(),
                indices: VecDeque::new(),
                index: 0,
                count,
            }
        }
    }

    impl<G: Generator> Iterator for Samples<G> {
        type Item = G::Item;

        fn next(&mut self) -> Option<Self::Item> {
            if self.index < self.count {
                let size = (self.index as f64 / self.count as f64).min(1.);
                let mut state = State {
                    random: take(&mut self.random),
                    size,
                    depth: 0,
                    iteration: self.index,
                    iterations: self.count,
                };
                let item = self.generator.generate(&mut state);
                self.index += 1;
                self.random = state.random;
                self.indices.clear();
                Some(item)
            } else {
                None
            }
        }
    }

    impl<G: Generator> ExactSizeIterator for Samples<G> {
        #[inline]
        fn len(&self) -> usize {
            self.count - self.index
        }
    }
}

pub mod shrinker {
    use super::*;

    pub trait IntoShrinker {
        type Item;
        type Shrinker: Shrinker<Item = Self::Item>;
        fn shrinker(self) -> Self::Shrinker;
    }

    pub trait Shrinker {
        type Item;
        type Generator: Generator<Item = Self::Item>;
        fn shrink(&mut self) -> Option<Self::Generator>;
    }
}

pub mod array {
    use super::*;

    macro_rules! array {
        ($t:ty, [$($n:ident)?]) => {
            impl<T: Clone $(, const $n: usize)?> Generator for $t {
                type Item = T;
                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self[(0..self.len()).generate(state)].clone()
                }
            }

            impl<T: Clone $(, const $n: usize)?> IntoShrinker for $t {
                type Item = T;
                type Shrinker = Self;
                fn shrinker(self) -> Self::Shrinker {
                    self
                }
            }

            impl<T: Clone $(, const $n: usize)?> Shrinker for $t {
                type Item = T;
                type Generator = Self;
                fn shrink(&mut self) -> Option<Self::Generator> {
                    Some(Clone::clone(self))
                }
            }
        };
    }

    array!([T; N], [N]);
    array!(&'_ [T; N], [N]);
    array!(&'_ [T], []);
}

pub mod function {
    use super::*;

    impl<T> Generator for fn() -> T {
        type Item = T;
        #[inline]
        fn generate(&mut self, _: &mut State) -> Self::Item {
            self()
        }
    }

    impl<T> Generator for fn(&mut State) -> T {
        type Item = T;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self(state)
        }
    }

    impl<T> IntoShrinker for fn() -> T {
        type Item = T;
        type Shrinker = Self;
        fn shrinker(self) -> Self::Shrinker {
            self
        }
    }

    impl<T> IntoShrinker for fn(&mut State) -> T {
        type Item = T;
        type Shrinker = Self;
        fn shrinker(self) -> Self::Shrinker {
            self
        }
    }

    impl<T> Shrinker for fn() -> T {
        type Item = T;
        type Generator = Self;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(*self)
        }
    }

    impl<T> Shrinker for fn(&mut State) -> T {
        type Item = T;
        type Generator = Self;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(*self)
        }
    }
}

pub mod option {
    use super::*;

    impl<G: IntoGenerator> IntoGenerator for Option<G> {
        type Item = Option<G::Item>;
        type Generator = Any<(Map<G::Generator, Self::Item>, fn() -> Self::Item)>;

        #[inline]
        fn generator() -> Self::Generator {
            let some: fn(G::Item) -> Self::Item = Some;
            let none: fn() -> Self::Item = || None;
            Any::from((G::generator().map(some), none))
        }
    }

    impl<G: Generator> Generator for Option<G> {
        type Item = Option<G::Item>;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            Some(self.as_mut()?.generate(state))
        }
    }

    impl<S: IntoShrinker> IntoShrinker for Option<S> {
        type Item = S::Item;
        type Shrinker = Option<S::Shrinker>;
        fn shrinker(self) -> Self::Shrinker {
            Some(self?.shrinker())
        }
    }

    impl<S: Shrinker> Shrinker for Option<S> {
        type Item = S::Item;
        type Generator = S::Generator;
        fn shrink(&mut self) -> Option<Self::Generator> {
            self.as_mut()?.shrink()
        }
    }
}

pub mod or {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub enum Or<L, R> {
        Left(L),
        Right(R),
    }

    impl<L: Generator, R: Generator<Item = L::Item>> Generator for Or<L, R> {
        type Item = L::Item;
        fn generate(&mut self, state: &mut State) -> Self::Item {
            match self {
                Or::Left(left) => left.generate(state),
                Or::Right(right) => right.generate(state),
            }
        }
    }

    impl<L: IntoShrinker, R: IntoShrinker<Item = L::Item>> IntoShrinker for Or<L, R> {
        type Item = L::Item;
        type Shrinker = Or<L::Shrinker, R::Shrinker>;
        fn shrinker(self) -> Self::Shrinker {
            match self {
                Or::Left(left) => Or::Left(left.shrinker()),
                Or::Right(right) => Or::Right(right.shrinker()),
            }
        }
    }

    impl<L: Shrinker, R: Shrinker<Item = L::Item>> Shrinker for Or<L, R> {
        type Item = L::Item;
        type Generator = Or<L::Generator, R::Generator>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            match self {
                Or::Left(left) => Some(Or::Left(left.shrink()?)),
                Or::Right(right) => Some(Or::Right(right.shrink()?)),
            }
        }
    }
}

pub mod map {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Map<G, T, F = fn(<G as Generator>::Item) -> T>(G, F, PhantomData<T>);
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Shrink<S, T, F>(S, F, PhantomData<T>);

    impl<G: Generator, T, F: FnMut(G::Item) -> T> Map<G, T, F> {
        pub fn new(generator: G, map: F) -> Self {
            Self(generator, map, PhantomData)
        }
    }

    impl<G: Generator, T, F: FnMut(G::Item) -> T> Generator for Map<G, T, F> {
        type Item = T;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self.1(self.0.generate(state))
        }
    }

    impl<S: IntoShrinker, T, F: FnMut(S::Item) -> T + Clone> IntoShrinker for Map<S, T, F> {
        type Item = T;
        type Shrinker = Shrink<S::Shrinker, T, F>;
        #[inline]
        fn shrinker(self) -> Self::Shrinker {
            Shrink(self.0.shrinker(), self.1, PhantomData)
        }
    }

    impl<S: Shrinker, T, F: FnMut(S::Item) -> T + Clone> Shrinker for Shrink<S, T, F> {
        type Item = T;
        type Generator = Map<S::Generator, T, F>;

        #[inline]
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(Map(self.0.shrink()?, self.1.clone(), PhantomData))
        }
    }
}

pub mod flatten {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Flatten<I, O>(O, Option<I>);
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Shrink<I, O>(O, Option<I>);

    impl<I: Generator, O: Generator<Item = I>> Flatten<I, O> {
        pub fn new(generator: O) -> Self {
            Self(generator, None)
        }
    }

    impl<I: Generator, O: Generator<Item = I>> Generator for Flatten<I, O> {
        type Item = I::Item;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            let mut generator = self.0.generate(state);
            let item = generator.generate(state);
            self.1 = Some(generator);
            item
        }
    }

    impl<I: IntoShrinker, O: IntoShrinker<Item = <I::Shrinker as Shrinker>::Generator>> IntoShrinker
        for Flatten<I, O>
    {
        type Item = I::Item;
        type Shrinker = Shrink<I::Shrinker, O::Shrinker>;
        fn shrinker(self) -> Self::Shrinker {
            Shrink(self.0.shrinker(), self.1.map(|inner| inner.shrinker()))
        }
    }

    impl<I: Shrinker, O: Shrinker<Item = I::Generator>> Shrinker for Shrink<I, O> {
        type Item = I::Item;
        type Generator = Or<Flatten<I::Generator, O::Generator>, I::Generator>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            match self.0.shrink() {
                Some(generator) => Some(Or::Left(generator.flatten())),
                None => self.1.shrink().map(Or::Right),
            }
        }
    }
}
