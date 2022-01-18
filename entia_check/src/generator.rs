use self::{
    array::Array, collect::Collect, constant::Constant, filter::Filter, filter_map::FilterMap,
    flatten::Flatten, map::Map, or::Or, sample::Sample, size::Size, wrap::Wrap,
};
use crate::{
    all::All,
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

// TODO: Review all 'shrink' implementations and ensure that only one 'shrink' happens per call (ex: tuples must shrink only 1 item at a time).
// TODO: Replace 'Generator' implementations that operate directly on values (such as 'Vec<T>' and '[T; N]') with 'IntoGenerator'
// implementations?
pub trait IntoGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;
    fn generator(self) -> Self::Generator;
}

pub trait Generator: Clone {
    type Item;
    type State;
    type Shrink: Generator<Item = Self::Item>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::State);
    fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink>;

    fn wrap<T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>(
        self,
        before: B,
        after: A,
    ) -> Wrap<Self, T, B, A>
    where
        Self: Sized,
    {
        Wrap::new(self, before, after)
    }

    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map::new(self, map)
    }

    fn filter<F: Fn(&Self::Item) -> bool + Clone>(
        self,
        iterations: Option<usize>,
        filter: F,
    ) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter::new(self, filter, iterations.unwrap_or(256))
    }

    fn filter_map<T, F: Fn(Self::Item) -> Option<T> + Clone>(
        self,
        iterations: Option<usize>,
        map: F,
    ) -> FilterMap<Self, T, F>
    where
        Self: Sized,
    {
        FilterMap::new(self, map, iterations.unwrap_or(256))
    }

    fn bind<G: Generator, F: Fn(Self::Item) -> G + Clone>(self, bind: F) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
    {
        self.map(bind).flatten()
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generator,
    {
        Flatten::new(self)
    }

    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        Array::new(self)
    }

    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Size<Range<usize>>, F>
    where
        Self: Sized,
    {
        self.collect_with((0..256 as usize).generator())
    }

    fn collect_with<C: Generator<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
    {
        Collect::new(self, count)
    }

    fn size(self) -> Size<Self>
    where
        Self: Sized,
        Size<Self>: Generator,
    {
        Size::new(self)
    }

    fn sample(&mut self, count: usize) -> Sample<Self>
    where
        Self: Sized,
    {
        Sample::new(self, State::new(count))
    }

    fn check<F: Fn(&Self::Item) -> bool>(&self, count: usize, check: F) -> Result<(), Self::Item> {
        fn shrink<G: Generator, F: Fn(&G::Item) -> bool>(
            generator: &G,
            mut pair: (G::Item, G::State),
            state: State,
            check: F,
        ) -> G::Item {
            while let Some(generator) = generator.shrink(&mut pair.1) {
                let pair = generator.generate(&mut state.clone());
                if !check(&pair.0) {
                    return shrink(&generator, pair, state, check);
                }
            }
            pair.0
        }

        // TODO: Parallelize checking!
        for mut state in State::new(count) {
            let old = state.clone();
            let pair = self.generate(&mut state);
            if !check(&pair.0) {
                return Err(shrink(self, pair, old, check));
            }
        }
        Ok(())
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
    fn generator() -> Self::Generator {
        char::generator().collect()
    }
}

impl<G: FullGenerator> FullGenerator for Vec<G> {
    type Item = Vec<G::Item>;
    type Generator = Collect<G::Generator, Size<Range<usize>>, Self::Item>;
    fn generator() -> Self::Generator {
        G::generator().collect()
    }
}

pub mod sample {
    use super::*;

    #[derive(Debug)]
    pub struct Sample<'a, G> {
        generator: &'a mut G,
        state: State,
    }

    impl<'a, G> Sample<'a, G> {
        pub fn new(generator: &'a mut G, state: State) -> Self {
            Self { generator, state }
        }
    }

    impl<G: Generator> Iterator for Sample<'_, G> {
        type Item = G::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.state = self.state.next()?;
            Some(self.generator.generate(&mut self.state).0)
        }
    }

    impl<G: Generator> ExactSizeIterator for Sample<'_, G> {
        #[inline]
        fn len(&self) -> usize {
            self.state.len()
        }
    }
}

pub mod size {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
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
            Self(generator)
        }
    }
}

pub mod constant {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Constant<T>(T);

    impl<T: Clone> Constant<T> {
        #[inline]
        pub fn new(value: T) -> Self {
            Self(value)
        }
    }

    impl<T: Clone> From<T> for Constant<T> {
        #[inline]
        fn from(value: T) -> Self {
            Self::new(value)
        }
    }

    impl<T: Clone> Generator for Constant<T> {
        type Item = T;
        type State = ();
        type Shrink = Self;

        fn generate(&self, _: &mut State) -> (Self::Item, Self::State) {
            (self.0.clone(), ())
        }

        fn shrink(&self, _: &mut Self::State) -> Option<Self::Shrink> {
            None
        }
    }
}

pub mod array {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Array<G, const N: usize>(G);

    impl<G: Generator, const N: usize> Array<G, N> {
        #[inline]
        pub fn new(generator: G) -> Self {
            Self(generator)
        }
    }

    impl<G: Generator, const N: usize> Generator for Array<G, N> {
        type Item = <All<[G; N]> as Generator>::Item;
        type State = <All<[G; N]> as Generator>::State;
        type Shrink = <All<[G; N]> as Generator>::Shrink;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let mut items = [(); N].map(|_| None);
            let mut states = [(); N].map(|_| None);
            for i in 0..N {
                let (item, state) = self.0.generate(state);
                items[i] = Some(item);
                states[i] = Some(state);
            }
            (items.map(Option::unwrap), (states.map(Option::unwrap), 0))
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            All::from([(); N].map(|_| self.0.clone())).shrink(state)
        }
    }

    macro_rules! array {
        ($t:ty, [$($n:ident)?]) => {
            impl<T: Clone $(, const $n: usize)?> Generator for $t {
                type Item = T;
                type State = usize;
                type Shrink = Constant<T>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let index = state.random.usize(0..self.len());
                    (self[index].clone(), index)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    Some(Constant::new(self[*state].clone()))
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
        type State = ();
        type Shrink = Self;

        fn generate(&self, _: &mut State) -> (Self::Item, Self::State) {
            (self(), ())
        }

        fn shrink(&self, _: &mut Self::State) -> Option<Self::Shrink> {
            None
        }
    }

    impl<T> Generator for fn(&mut State) -> T {
        type Item = T;
        type State = ();
        type Shrink = Self;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            (self(state), ())
        }

        fn shrink(&self, _: &mut Self::State) -> Option<Self::Shrink> {
            None
        }
    }
}

pub mod option {
    use super::*;

    impl<G: FullGenerator> FullGenerator for Option<G> {
        type Item = Option<G::Item>;
        type Generator = Any<(Map<G::Generator, Self::Item>, fn() -> Self::Item)>;
        fn generator() -> Self::Generator {
            let some: fn(G::Item) -> Self::Item = Some;
            let none: fn() -> Self::Item = || None;
            Any::from((G::generator().map(some), none))
        }
    }

    impl<G: Generator> Generator for Option<G> {
        type Item = Option<G::Item>;
        type State = Option<G::State>;
        type Shrink = Option<G::Shrink>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            match self {
                Some(generator) => {
                    let (item, state) = generator.generate(state);
                    (Some(item), Some(state))
                }
                None => (None, None),
            }
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(self.as_ref()?.shrink(state.as_mut()?))
        }
    }
}

pub mod or {
    use super::*;
    use Or::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub enum Or<L, R> {
        Left(L),
        Right(R),
    }

    impl<L, R> Or<Or<L, R>, Or<L, R>> {
        #[inline]
        pub fn flatten(self) -> Or<L, R> {
            match self {
                Left(Left(left)) => Left(left),
                Left(Right(right)) => Right(right),
                Right(Left(left)) => Left(left),
                Right(Right(right)) => Right(right),
            }
        }
    }

    impl<L, R> Or<Or<L, R>, R> {
        #[inline]
        pub fn flatten_left(self) -> Or<L, R> {
            match self {
                Left(Left(left)) => Left(left),
                Left(Right(right)) => Right(right),
                Right(right) => Right(right),
            }
        }
    }

    impl<L, R> Or<L, Or<L, R>> {
        #[inline]
        pub fn flatten_right(self) -> Or<L, R> {
            match self {
                Left(left) => Left(left),
                Right(Left(left)) => Left(left),
                Right(Right(right)) => Right(right),
            }
        }
    }

    impl<L: Generator, R: Generator<Item = L::Item>> Generator for Or<L, R> {
        type Item = L::Item;
        type State = Or<L::State, R::State>;
        type Shrink = Or<L::Shrink, R::Shrink>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            match self {
                Left(generator) => {
                    let (item, state) = generator.generate(state);
                    (item, Left(state))
                }
                Right(generator) => {
                    let (item, state) = generator.generate(state);
                    (item, Right(state))
                }
            }
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            match (self, state) {
                (Left(generator), Left(state)) => Some(Left(generator.shrink(state)?)),
                (Right(generator), Right(state)) => Some(Right(generator.shrink(state)?)),
                _ => None,
            }
        }
    }
}

pub mod wrap {
    use super::*;

    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Wrap<G, T, B = fn(&mut State) -> T, A = fn(&mut State, T)>(G, B, A, PhantomData<T>);

    impl<G: Clone, T, B: Clone, A: Clone> Clone for Wrap<G, T, B, A> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone(), self.2.clone(), PhantomData)
        }
    }

    impl<G: Generator, T, B: FnMut(&mut State) -> T + Clone, A: FnMut(&mut State, T) + Clone>
        Wrap<G, T, B, A>
    {
        #[inline]
        pub fn new(generator: G, before: B, after: A) -> Self {
            Self(generator, before, after, PhantomData)
        }
    }

    impl<G: Generator, T, B: Fn(&mut State) -> T + Clone, A: Fn(&mut State, T) + Clone> Generator
        for Wrap<G, T, B, A>
    {
        type Item = G::Item;
        type State = G::State;
        type Shrink = Wrap<G::Shrink, T, B, A>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let value = self.1(state);
            let pair = self.0.generate(state);
            self.2(state, value);
            pair
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(self.0.shrink(state)?.wrap(self.1.clone(), self.2.clone()))
        }
    }
}

pub mod map {
    use super::*;

    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Map<G, T, F = fn(<G as Generator>::Item) -> T>(G, F, PhantomData<T>);

    impl<G: Clone, T, F: Clone> Clone for Map<G, T, F> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone(), PhantomData)
        }
    }

    impl<G: Generator, T, F: Fn(G::Item) -> T + Clone> Map<G, T, F> {
        #[inline]
        pub fn new(generator: G, map: F) -> Self {
            Self(generator, map, PhantomData)
        }
    }

    impl<G: Generator, T, F: Fn(G::Item) -> T + Clone> Generator for Map<G, T, F> {
        type Item = T;
        type State = G::State;
        type Shrink = Map<G::Shrink, T, F>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let (item, state) = self.0.generate(state);
            (self.1(item), state)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(Map(self.0.shrink(state)?, self.1.clone(), PhantomData))
        }
    }
}

pub mod flatten {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Flatten<G: Generator>(G);

    impl<G: Generator<Item = impl Generator>> Flatten<G> {
        pub fn new(generator: G) -> Self {
            Self(generator)
        }
    }

    impl<G: Generator<Item = impl Generator>> Generator for Flatten<G> {
        type Item = <G::Item as Generator>::Item;
        type State = (G::State, G::Item, <G::Item as Generator>::State);
        // TODO: Fix infinite type (because of 'Or::Left').
        type Shrink = Or<Flatten<G::Shrink>, <<G as Generator>::Item as Generator>::Shrink>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let pair1 = self.0.generate(state);
            let pair2 = pair1.0.generate(state);
            (pair2.0, (pair1.1, pair1.0, pair2.1))
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            match state.1.shrink(&mut state.2) {
                Some(generator) => Some(Or::Right(generator)),
                None => Some(Or::Left(self.0.shrink(&mut state.0)?.flatten())),
            }
        }
    }
}

pub mod collect {
    use super::*;

    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Collect<G, C, F>(G, C, PhantomData<F>);

    impl<G: Clone, C: Clone, F> Clone for Collect<G, C, F> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone(), PhantomData)
        }
    }

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
        type State = (Vec<G::State>, usize, usize);
        type Shrink = Map<<All<Vec<G>> as Generator>::Shrink, F>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let (count, _) = self.1.generate(state);
            let mut states = Vec::with_capacity(count);
            let items = Iterator::map(0..count, |_| {
                let (item, state) = self.0.generate(state);
                states.push(state);
                item
            })
            .collect();
            (items, (states, 0, 0))
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(
                All::from(vec![self.0.clone(); state.0.len()])
                    .shrink(state)?
                    .map(|items| items.into_iter().collect()),
            )
        }
    }
}

pub mod filter {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Filter<G, F = fn(&<G as Generator>::Item) -> bool>(G, F, usize);

    impl<G: Generator, F: Fn(&G::Item) -> bool + Clone> Filter<G, F> {
        #[inline]
        pub fn new(generator: G, filter: F, iterations: usize) -> Self {
            Self(generator, filter, iterations)
        }
    }

    impl<G: Generator, F: Fn(&G::Item) -> bool + Clone> Generator for Filter<G, F> {
        type Item = Option<G::Item>;
        type State = Option<G::State>;
        type Shrink = Filter<G::Shrink, F>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            for _ in 0..self.2 {
                let (item, state) = self.0.generate(state);
                if self.1(&item) {
                    return (Some(item), Some(state));
                }
            }
            (None, None)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(Filter(
                self.0.shrink(state.as_mut()?)?,
                self.1.clone(),
                self.2,
            ))
        }
    }
}

pub mod filter_map {
    use super::*;

    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct FilterMap<G, T, F = fn(<G as Generator>::Item) -> Option<T>>(
        G,
        F,
        usize,
        PhantomData<T>,
    );

    impl<G: Clone, T, F: Clone> Clone for FilterMap<G, T, F> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone(), self.2, PhantomData)
        }
    }

    impl<G: Generator, T, F: Fn(G::Item) -> Option<T> + Clone> FilterMap<G, T, F> {
        #[inline]
        pub fn new(generator: G, map: F, iterations: usize) -> Self {
            Self(generator, map, iterations, PhantomData)
        }
    }

    impl<G: Generator, T, F: Fn(G::Item) -> Option<T> + Clone> Generator for FilterMap<G, T, F> {
        type Item = Option<T>;
        type State = Option<G::State>;
        type Shrink = FilterMap<G::Shrink, T, F>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            for _ in 0..self.2 {
                let (item, state) = self.0.generate(state);
                if let Some(item) = self.1(item) {
                    return (Some(item), Some(state));
                }
            }
            (None, None)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            Some(FilterMap(
                self.0.shrink(state.as_mut()?)?,
                self.1.clone(),
                self.2,
                PhantomData,
            ))
        }
    }
}
