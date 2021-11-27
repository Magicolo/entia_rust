use crate::primitive::Full;
use entia_core::utility;
use fastrand::Rng;
use std::{iter::FromIterator, marker::PhantomData, mem::take, ops::Range};

pub trait IntoGenerator {
    type Item;
    type Generator: Generator<Item = Self::Item>;

    fn generator() -> Self::Generator;

    #[inline]
    fn generate(state: &mut State) -> Self::Item {
        Self::generator().generate(state)
    }
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
    fn map<T, F: FnMut(Self::Item) -> T>(self, map: F) -> With<(Self, F), T>
    where
        Self: Sized,
    {
        With::new((self, map), |state, (generator, map)| {
            Some(map(generator.generate(state)))
        })
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
    fn bind<G: Generator, F: FnMut(Self::Item) -> G>(self, bind: F) -> With<(Self, F), G::Item>
    where
        Self: Sized,
    {
        With::new((self, bind), |state, (generator, bind)| {
            Some(bind(generator.generate(state)).generate(state))
        })
    }

    #[inline]
    fn flatten(self) -> With<Self, <Self::Item as Generator>::Item>
    where
        Self: Sized,
        Self::Item: Generator,
    {
        With::new(self, |state, generator| {
            Some(generator.generate(state).generate(state))
        })
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
        Samples {
            generator: self,
            index: 0,
            count,
            random: Rng::new(),
        }
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

#[derive(Debug, Clone)]
pub struct Samples<G> {
    generator: G,
    random: Rng,
    index: usize,
    count: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct With<S, T, F = fn(&mut State, &mut S) -> Option<T>>(S, F, PhantomData<T>);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size<G>(pub G);

impl<S, T, F: FnMut(&mut State, &mut S) -> Option<T>> With<S, T, F> {
    #[inline]
    pub fn new(state: S, with: F) -> Self {
        Self(state, with, PhantomData)
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

impl<G: IntoGenerator> IntoGenerator for Box<[G]> {
    type Item = Box<[G::Item]>;
    type Generator = With<(G::Generator, Size<Range<usize>>), Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        G::generator().collect()
    }
}

impl<T: IntoGenerator> IntoGenerator for Option<T> {
    type Item = Option<T::Item>;
    type Generator = With<T::Generator, Self::Item>;

    #[inline]
    fn generator() -> Self::Generator {
        With::new(T::generator(), |state, some| {
            Some(if state.random.bool() {
                Some(some.generate(state))
            } else {
                None
            })
        })
    }
}

impl<T: IntoGenerator, E: IntoGenerator> IntoGenerator for Result<T, E> {
    type Item = Result<T::Item, E::Item>;
    type Generator = With<(T::Generator, E::Generator), Self::Item>;

    #[inline]
    fn generator() -> Self::Generator {
        With::new((T::generator(), E::generator()), |state, (ok, err)| {
            Some(if state.random.bool() {
                Ok(ok.generate(state))
            } else {
                Err(err.generate(state))
            })
        })
    }
}

impl<S, T, F: FnMut(&mut State, &mut S) -> Option<T>> Generator for With<S, T, F> {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        loop {
            match self.1(state, &mut self.0) {
                Some(value) => return value,
                None => {}
            }
        }
    }
}

impl<T: Clone, const N: usize> Generator for [T; N] {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self[(0..N).generate(state)].clone()
    }
}

impl<T: Clone, const N: usize> Generator for &'_ [T; N] {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self[(0..N).generate(state)].clone()
    }
}

impl<T: Clone> Generator for [T] {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self[(0..self.len()).generate(state)].clone()
    }
}

impl<'a, T: Clone> Generator for &'_ [T] {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self[(0..self.len()).generate(state)].clone()
    }
}

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
