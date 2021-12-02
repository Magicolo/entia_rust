use self::{
    adapt::Adapt, array::Array, collect::Collect, flatten::Flatten, map::Map, or::Or,
    sample::Samples, size::Size,
};
use crate::{any::Any, primitive::Full};
use fastrand::Rng;
use std::{
    collections::VecDeque,
    iter::FromIterator,
    marker::PhantomData,
    mem::take,
    ops::Range,
    ops::{Deref, DerefMut},
};

pub trait IntoGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn generator() -> Self::Generator;
}

pub trait Generator {
    type Item;
    fn generate(&mut self, state: &mut State) -> Self::Item;

    #[inline]
    fn adapt<T, B: FnMut(&mut State) -> T, A: FnMut(&mut State, T)>(
        self,
        before: B,
        after: A,
    ) -> Adapt<Self, T, B, A>
    where
        Self: Sized,
    {
        Adapt::new(self, before, after)
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
    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        Array::new(self)
    }

    #[inline]
    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Size<Range<usize>>, F>
    where
        Self: Sized,
    {
        self.collect_with((0..256).size())
    }

    #[inline]
    fn collect_with<C: Generator<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
    {
        Collect::new(self, count)
    }

    #[inline]
    fn size(self) -> Size<Self>
    where
        Self: Sized,
    {
        Size::new(self)
    }

    #[inline]
    fn sample(self, count: usize) -> Samples<Self>
    where
        Self: Sized,
    {
        Samples::new(self, count)
    }
}

pub trait Shrinker {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn shrink(&mut self) -> Option<Self::Generator>;
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

impl<S, T, F: FnMut(&mut State, &mut S) -> Option<T>> With<S, T, F> {
    #[inline]
    pub fn new(state: S, with: F) -> Self {
        Self(state, with, PhantomData)
    }
}

impl IntoGenerator for String {
    type Item = Self;
    type Generator = Collect<Size<Full<char>>, Size<Range<usize>>, Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        char::generator().collect()
    }
}

impl<G: IntoGenerator> IntoGenerator for Vec<G> {
    type Item = Vec<G::Item>;
    type Generator = Collect<G::Generator, Size<Range<usize>>, Self::Item>;
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

pub mod size {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Size<G>(G);

    impl<T> Deref for Size<T> {
        type Target = T;
        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Size<T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<G: Generator> Size<G> {
        #[inline]
        pub fn new(generator: G) -> Self {
            Size(generator)
        }
    }

    impl<S: Shrinker> Shrinker for Size<S> {
        type Item = S::Item;
        type Generator = S::Generator;
        fn shrink(&mut self) -> Option<Self::Generator> {
            self.0.shrink()
        }
    }
}

pub mod array {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Array<G, const N: usize>(G);

    impl<G: Generator, const N: usize> Array<G, N> {
        pub fn new(generator: G) -> Self {
            Self(generator)
        }
    }

    impl<G: Generator, const N: usize> Generator for Array<G, N> {
        type Item = [G::Item; N];
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            [(); N].map(|_| self.0.generate(state))
        }
    }

    impl<S: Shrinker, const N: usize> Shrinker for Array<S, N> {
        type Item = [S::Item; N];
        type Generator = Array<S::Generator, N>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(Array(self.0.shrink()?))
        }
    }

    macro_rules! array {
        ($t:ty, [$($n:ident)?]) => {
            impl<T: Clone $(, const $n: usize)?> Generator for $t {
                type Item = T;
                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self[(0..self.len()).generate(state)].clone()
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

pub mod adapt {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Adapt<G, T, B = fn(&mut State) -> T, A = fn(&mut State, T)>(G, B, A, PhantomData<T>);

    impl<G: Generator, T, B: FnMut(&mut State) -> T, A: FnMut(&mut State, T)> Adapt<G, T, B, A> {
        #[inline]
        pub fn new(generator: G, before: B, after: A) -> Self {
            Self(generator, before, after, PhantomData)
        }
    }

    impl<G: Generator, T, B: FnMut(&mut State) -> T, A: FnMut(&mut State, T)> Generator
        for Adapt<G, T, B, A>
    {
        type Item = G::Item;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            let value = self.1(state);
            let item = self.0.generate(state);
            self.2(state, value);
            item
        }
    }

    impl<S: Shrinker, T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>
        Shrinker for Adapt<S, T, B, A>
    {
        type Item = S::Item;
        type Generator = Adapt<S::Generator, T, B, A>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(self.0.shrink()?.adapt(self.1.clone(), self.2.clone()))
        }
    }
}

pub mod map {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Map<G, T, F = fn(<G as Generator>::Item) -> T>(G, F, PhantomData<T>);

    impl<G: Generator, T, F: FnMut(G::Item) -> T> Map<G, T, F> {
        #[inline]
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

    impl<S: Shrinker, T, F: FnMut(S::Item) -> T + Clone> Shrinker for Map<S, T, F> {
        type Item = T;
        type Generator = Map<S::Generator, T, F>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(Map(self.0.shrink()?, self.1.clone(), PhantomData))
        }
    }
}

pub mod flatten {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Flatten<I, O>(O, Option<I>);

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

    impl<I: Shrinker, O: Shrinker<Item = I::Generator>> Shrinker for Flatten<I, O> {
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

pub mod collect {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Collect<G, C, F>(G, C, PhantomData<F>);

    impl<G: Generator, C: Generator<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
        #[inline]
        pub fn new(generator: G, count: C) -> Self {
            Self(generator, count, PhantomData)
        }
    }

    impl<G: Generator, C: Generator<Item = usize>, F: FromIterator<G::Item>> Generator
        for Collect<G, C, F>
    {
        type Item = F;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            Iterator::map(0..self.1.generate(state), |_| self.0.generate(state)).collect()
        }
    }

    impl<S: Shrinker, C: Shrinker<Item = usize>, F: FromIterator<S::Item>> Shrinker
        for Collect<S, C, F>
    {
        type Item = F;
        type Generator = Collect<S::Generator, C::Generator, F>;
        fn shrink(&mut self) -> Option<Self::Generator> {
            Some(Collect(self.0.shrink()?, self.1.shrink()?, PhantomData))
        }
    }
}

pub mod filter {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Filter<G, F = fn(<G as Generator>::Item) -> bool>(G, F);

    // TODO: complete
}

pub mod filter_map {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct FilterMap<G, T, F = fn(<G as Generator>::Item) -> Option<T>>(G, F, PhantomData<T>);

    // TODO: complete
}
