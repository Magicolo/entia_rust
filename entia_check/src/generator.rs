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
    fn adapt<F: FnMut(&mut State)>(self, adapt: F) -> Adapt<Self, F>
    where
        Self: Sized,
    {
        Adapt(self, adapt)
    }

    #[inline]
    fn map<T, F: FnMut(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map(self, map, PhantomData)
    }

    #[inline]
    fn bind<G: Generator, F: FnMut(Self::Item) -> G>(self, bind: F) -> Flatten<Map<Self, G, F>>
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
        Flatten(self)
    }

    #[inline]
    fn sample(self, count: usize) -> Sample<Self>
    where
        Self: Sized,
    {
        Sample {
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
}

pub struct Sample<G> {
    generator: G,
    random: Rng,
    index: usize,
    count: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Constant<T>(pub T);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Map<G, T, F = fn(<G as Generator>::Item) -> T>(pub G, pub F, PhantomData<T>);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Adapt<G, F = fn(&mut State)>(pub G, pub F);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Flatten<G>(pub G);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct With<T, F = fn(&mut State) -> T>(pub F, PhantomData<T>);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Iterate<G, I>(pub I, PhantomData<G>);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size<G>(pub G);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Many<C, G, F>(pub C, pub G, PhantomData<F>);
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Count;

impl Count {
    pub const RANGE: Range<usize> = 0..256;
}

impl<T, F: FnMut(&mut State) -> T> With<T, F> {
    #[inline]
    pub fn new(with: F) -> Self {
        With(with, PhantomData)
    }
}

impl<G: Generator> Iterator for Sample<G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let size = (self.index as f64 / self.count as f64).min(1.);
            self.index += 1;
            let mut state = State {
                random: take(&mut self.random),
                size,
                depth: 0,
            };
            let item = self.generator.generate(&mut state);
            self.random = state.random;
            Some(item)
        } else {
            None
        }
    }
}

impl<G: Generator> ExactSizeIterator for Sample<G> {
    #[inline]
    fn len(&self) -> usize {
        self.count - self.index
    }
}

impl<G: IntoGenerator, F: FromIterator<G::Item>> IntoGenerator for Iterate<G, F> {
    type Item = F;
    type Generator = Many<Count, G::Generator, Self::Item>;
    #[inline]
    fn generator() -> Self::Generator {
        Many(Count, G::generator(), PhantomData)
    }
}

impl IntoGenerator for Count {
    type Item = usize;
    type Generator = Self;
    #[inline]
    fn generator() -> Self::Generator {
        Self
    }
}

impl IntoGenerator for String {
    type Item = Self;
    type Generator = <Iterate<char, Self::Item> as IntoGenerator>::Generator;
    #[inline]
    fn generator() -> Self::Generator {
        <Iterate<char, Self::Item> as IntoGenerator>::generator()
    }
}

impl<G: IntoGenerator> IntoGenerator for Vec<G> {
    type Item = Vec<G::Item>;
    type Generator = <Iterate<G, Self::Item> as IntoGenerator>::Generator;
    #[inline]
    fn generator() -> Self::Generator {
        <Iterate<G, Self::Item> as IntoGenerator>::generator()
    }
}

impl<G: IntoGenerator> IntoGenerator for Box<[G]> {
    type Item = Box<[G::Item]>;
    type Generator = <Iterate<G, Self::Item> as IntoGenerator>::Generator;
    #[inline]
    fn generator() -> Self::Generator {
        <Iterate<G, Self::Item> as IntoGenerator>::generator()
    }
}

impl<T, F: FnMut(&mut State) -> T> Generator for With<T, F> {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        (self.0)(state)
    }
}

impl<T: Clone> Generator for Constant<T> {
    type Item = T;
    #[inline]
    fn generate(&mut self, _: &mut State) -> Self::Item {
        self.0.clone()
    }
}

impl Generator for Count {
    type Item = usize;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Count::RANGE.clone().generate(state)
    }
}

impl Generator for Size<Count> {
    type Item = usize;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Size(Count::RANGE).generate(state)
    }
}

impl<G: Generator, T, F: FnMut(G::Item) -> T> Generator for Map<G, T, F> {
    type Item = T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self.1(self.0.generate(state))
    }
}

impl<G: Generator<Item = impl Generator>> Generator for Flatten<G> {
    type Item = <G::Item as Generator>::Item;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self.0.generate(state).generate(state)
    }
}

impl<G: Generator, F: FnMut(&mut State)> Generator for Adapt<G, F> {
    type Item = G::Item;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        self.1(state);
        self.0.generate(state)
    }
}

impl<G: Generator, I: Iterator<Item = G>> Generator for Iterate<G, I> {
    type Item = Option<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Some(self.0.next()?.generate(state))
    }
}

impl<T: Clone, const N: usize> Generator for [T; N] {
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

impl<G: Generator> Generator for &'_ mut G {
    type Item = G::Item;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        (*self).generate(state)
    }
}

impl<'a, T> Generator for &'a [T] {
    type Item = &'a T;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        &self[(0..self.len()).generate(state)]
    }
}

impl<C: Generator<Item = usize>, G: Generator, F: FromIterator<G::Item>> Generator
    for Many<C, G, F>
{
    type Item = F;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        Iterator::map(0..self.0.generate(state), |_| self.1.generate(state)).collect()
    }
}
