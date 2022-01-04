use self::{
    adapt::Adapt, array::Array, collect::Collect, filter::Filter, filter_map::FilterMap,
    flatten::Flatten, map::Map, or::Or, sample::Samples, size::Size,
};
use crate::{
    any::Any,
    primitive::{Full, Range},
};
use fastrand::Rng;
use std::{
    iter::FromIterator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub trait FullGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn generator() -> Self::Generator;
}

// TODO: Replace 'Generator' implementations that operate directly on values (such as 'Vec<T>' and '[T; N]') with 'IntoGenerator'
// implementations?
pub trait IntoGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn generator(self) -> Self::Generator;
}

pub trait Generator {
    type Item;
    type Shrink: Generator<Item = Self::Item>;
    fn generate(&mut self, state: &mut State) -> Self::Item;
    // TODO: Can 'shrink' take '&self'?
    fn shrink(&mut self) -> Option<Self::Shrink>;

    #[inline]
    fn adapt<T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>(
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
    fn map<T, F: FnMut(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map::new(self, map)
    }

    #[inline]
    fn filter<F: FnMut(&Self::Item) -> bool + Clone>(
        self,
        iterations: Option<usize>,
        filter: F,
    ) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter::new(self, filter, iterations.unwrap_or(256))
    }

    #[inline]
    fn filter_map<T, F: FnMut(Self::Item) -> Option<T> + Clone>(
        self,
        iterations: Option<usize>,
        map: F,
    ) -> FilterMap<Self, T, F>
    where
        Self: Sized,
    {
        FilterMap::new(self, map, iterations.unwrap_or(256))
    }

    #[inline]
    fn bind<G: Generator, F: FnMut(Self::Item) -> G + Clone>(
        self,
        bind: F,
    ) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
    {
        self.map(bind).flatten()
    }

    #[inline]
    fn flatten(self) -> Flatten<Self>
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
        self.collect_with((0..256 as usize).generator())
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
        Size<Self>: Generator,
    {
        Size::new(self)
    }

    #[inline]
    fn sample(&mut self, count: usize) -> Samples<Self>
    where
        Self: Sized,
    {
        Samples::new(self, State::new(count))
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub random: Rng,
    pub size: f64,
    pub depth: usize,
    pub index: usize,
    pub count: usize,
}

impl State {
    pub fn new(count: usize) -> Self {
        Self {
            random: Rng::new(),
            size: 0.,
            depth: 0,
            index: 0,
            count,
        }
    }
}

impl Iterator for State {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            // 10% of states will have size 1.
            self.size = (self.index as f64 / self.count as f64 * 1.1).min(1.);
            self.index += 1;
            Some(self.clone())
        } else {
            None
        }
    }
}

impl ExactSizeIterator for State {
    #[inline]
    fn len(&self) -> usize {
        self.count - self.index
    }
}

impl FullGenerator for String {
    type Item = Self;
    type Generator = Collect<Size<Full<char>>, Size<Range<usize>>, Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        char::generator().collect()
    }
}

impl<G: FullGenerator> FullGenerator for Vec<G> {
    type Item = Vec<G::Item>;
    type Generator = Collect<G::Generator, Size<Range<usize>>, Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        G::generator().collect()
    }
}

pub mod sample {
    use super::*;

    #[derive(Debug)]
    pub struct Samples<'a, G> {
        generator: &'a mut G,
        state: State,
    }

    impl<'a, G> Samples<'a, G> {
        pub fn new(generator: &'a mut G, state: State) -> Self {
            Self { generator, state }
        }
    }

    impl<G: Generator> Iterator for Samples<'_, G> {
        type Item = G::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.state = self.state.next()?;
            Some(self.generator.generate(&mut self.state))
        }
    }

    impl<G: Generator> ExactSizeIterator for Samples<'_, G> {
        #[inline]
        fn len(&self) -> usize {
            self.state.len()
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
}

pub mod array {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Array<G, const N: usize>(G);

    impl<G: Generator, const N: usize> Array<G, N> {
        // The compiler fails to detect the usage of this function and wrongly produces a warning.
        #[allow(dead_code)]
        #[inline]
        pub fn new(generator: G) -> Self {
            Self(generator)
        }
    }

    impl<G: Generator, const N: usize> Generator for Array<G, N> {
        type Item = [G::Item; N];
        type Shrink = Array<G::Shrink, N>;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            [(); N].map(|_| self.0.generate(state))
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(Array(self.0.shrink()?))
        }
    }

    macro_rules! array {
        ($t:ty, [$($n:ident)?]) => {
            impl<T: Clone $(, const $n: usize)?> Generator for $t {
                type Item = T;
                type Shrink = Self;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self[state.random.usize(0..self.len())].clone()
                }

                #[inline]
                fn shrink(&mut self) -> Option<Self::Shrink> {
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
        type Shrink = Self;
        #[inline]
        fn generate(&mut self, _: &mut State) -> Self::Item {
            self()
        }
        #[inline]
        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(*self)
        }
    }

    impl<T> Generator for fn(&mut State) -> T {
        type Item = T;
        type Shrink = Self;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self(state)
        }
        #[inline]
        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(*self)
        }
    }
}

pub mod option {
    use super::*;

    impl<G: FullGenerator> FullGenerator for Option<G> {
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
        type Shrink = Option<G::Shrink>;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            Some(self.as_mut()?.generate(state))
        }
        #[inline]
        fn shrink(&mut self) -> Option<Self::Shrink> {
            self.as_mut().map(G::shrink)
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
        type Shrink = Or<L::Shrink, R::Shrink>;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            match self {
                Or::Left(left) => left.generate(state),
                Or::Right(right) => right.generate(state),
            }
        }
        #[inline]
        fn shrink(&mut self) -> Option<Self::Shrink> {
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

    impl<G: Generator, T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>
        Adapt<G, T, B, A>
    {
        #[inline]
        pub fn new(generator: G, before: B, after: A) -> Self {
            Self(generator, before, after, PhantomData)
        }
    }

    impl<G: Generator, T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>
        Generator for Adapt<G, T, B, A>
    {
        type Item = G::Item;
        type Shrink = Adapt<G::Shrink, T, B, A>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            let value = self.1(state);
            let item = self.0.generate(state);
            self.2(state, value);
            item
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(self.0.shrink()?.adapt(self.1.clone(), self.2.clone()))
        }
    }
}

pub mod map {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Map<G, T, F = fn(<G as Generator>::Item) -> T>(G, F, PhantomData<T>);

    impl<G: Generator, T, F: FnMut(G::Item) -> T + Clone> Map<G, T, F> {
        #[inline]
        pub fn new(generator: G, map: F) -> Self {
            Self(generator, map, PhantomData)
        }
    }

    impl<G: Generator, T, F: FnMut(G::Item) -> T + Clone> Generator for Map<G, T, F> {
        type Item = T;
        type Shrink = Map<G::Shrink, T, F>;
        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self.1(self.0.generate(state))
        }
        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(Map(self.0.shrink()?, self.1.clone(), PhantomData))
        }
    }
}

pub mod flatten {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Flatten<G: Generator>(G, Option<G::Item>);

    impl<G: Generator<Item = impl Generator>> Flatten<G> {
        pub fn new(generator: G) -> Self {
            Self(generator, None)
        }
    }

    impl<G: Generator<Item = impl Generator>> Generator for Flatten<G> {
        type Item = <G::Item as Generator>::Item;
        type Shrink = Or<Flatten<G::Shrink>, <<G as Generator>::Item as Generator>::Shrink>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            let mut generator = self.0.generate(state);
            let item = generator.generate(state);
            self.1 = Some(generator);
            item
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            match self.0.shrink() {
                Some(generator) => Some(Or::Left(generator.flatten())),
                None => self.1.shrink()?.map(Or::Right),
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
        type Shrink = Collect<G::Shrink, C::Shrink, F>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            Iterator::map(0..self.1.generate(state), |_| self.0.generate(state)).collect()
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(Collect(self.0.shrink()?, self.1.shrink()?, PhantomData))
        }
    }
}

pub mod filter {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct Filter<G, F = fn(&<G as Generator>::Item) -> bool>(G, F, usize);

    impl<G: Generator, F: FnMut(&G::Item) -> bool + Clone> Filter<G, F> {
        // The compiler fails to detect the usage of this function and wrongly produces a warning.
        #[allow(dead_code)]
        #[inline]
        pub fn new(generator: G, filter: F, iterations: usize) -> Self {
            Self(generator, filter, iterations)
        }
    }

    impl<G: Generator, F: FnMut(&G::Item) -> bool + Clone> Generator for Filter<G, F> {
        type Item = Option<G::Item>;
        type Shrink = Filter<G::Shrink, F>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            for _ in 0..self.2 {
                let item = self.0.generate(state);
                if self.1(&item) {
                    return Some(item);
                }
            }
            None
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(Filter(self.0.shrink()?, self.1.clone(), self.2))
        }
    }
}

pub mod filter_map {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
    pub struct FilterMap<G, T, F = fn(<G as Generator>::Item) -> Option<T>>(
        G,
        F,
        usize,
        PhantomData<T>,
    );

    impl<G: Generator, T, F: FnMut(G::Item) -> Option<T> + Clone> FilterMap<G, T, F> {
        // The compiler fails to detect the usage of this function and wrongly produces a warning.
        #[allow(dead_code)]
        #[inline]
        pub fn new(generator: G, map: F, iterations: usize) -> Self {
            Self(generator, map, iterations, PhantomData)
        }
    }

    impl<G: Generator, T, F: FnMut(G::Item) -> Option<T> + Clone> Generator for FilterMap<G, T, F> {
        type Item = Option<T>;
        type Shrink = FilterMap<G::Shrink, T, F>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            for _ in 0..self.2 {
                if let Some(item) = self.1(self.0.generate(state)) {
                    return Some(item);
                }
            }
            None
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            Some(FilterMap(
                self.0.shrink()?,
                self.1.clone(),
                self.2,
                PhantomData,
            ))
        }
    }
}
