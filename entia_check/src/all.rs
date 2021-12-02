use crate::generator::{
    shrinker::{IntoShrinker, Shrinker},
    Generator, IntoGenerator, State,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct All<T>(T);

impl<G: IntoGenerator, const N: usize> IntoGenerator for [G; N] {
    type Item = [G::Item; N];
    type Generator = All<[G::Generator; N]>;
    #[inline]
    fn generator() -> Self::Generator {
        All([(); N].map(|_| G::generator()))
    }
}

impl<G: Generator, const N: usize> From<[G; N]> for All<[G; N]> {
    fn from(generators: [G; N]) -> Self {
        Self(generators)
    }
}

impl<G: Generator, const N: usize> Generator for All<[G; N]> {
    type Item = [G::Item; N];
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let mut index = 0;
        [(); N].map(|_| {
            let item = self.0[index].generate(state);
            index += 1;
            item
        })
    }
}

impl<S: IntoShrinker, const N: usize> IntoShrinker for All<[S; N]> {
    type Item = [S::Item; N];
    type Shrinker = All<[S::Shrinker; N]>;
    fn shrinker(self) -> Self::Shrinker {
        All(self.0.map(|shrinker| shrinker.shrinker()))
    }
}

impl<S: Shrinker, const N: usize> Shrinker for All<[S; N]> {
    type Item = [S::Item; N];
    type Generator = All<[S::Generator; N]>;
    fn shrink(&mut self) -> Option<Self::Generator> {
        let mut index = 0;
        let generators = [(); N].map(|_| {
            let generator = self.0[index].shrink();
            index += 1;
            generator
        });
        for generator in &generators {
            generator.as_ref()?;
        }
        Some(All(generators.map(Option::unwrap)))
    }
}

impl<G: Generator> From<Vec<G>> for All<Vec<G>> {
    fn from(generators: Vec<G>) -> Self {
        Self(generators)
    }
}

impl<G: Generator> Generator for All<Vec<G>> {
    type Item = Vec<G::Item>;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let map = |generator: &mut G| generator.generate(state);
        self.0.iter_mut().map(map).collect()
    }
}

impl<S: IntoShrinker> IntoShrinker for All<Vec<S>> {
    type Item = Vec<S::Item>;
    type Shrinker = All<Vec<S::Shrinker>>;
    fn shrinker(self) -> Self::Shrinker {
        All(self.0.into_iter().map(IntoShrinker::shrinker).collect())
    }
}

impl<S: Shrinker> Shrinker for All<Vec<S>> {
    type Item = Vec<S::Item>;
    type Generator = All<Vec<S::Generator>>;
    fn shrink(&mut self) -> Option<Self::Generator> {
        let mut generators = Vec::with_capacity(self.0.len());
        for shrinker in &mut self.0 {
            generators.push(shrinker.shrink()?);
        }
        Some(All(generators))
    }
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: IntoGenerator,)*> IntoGenerator for ($($t,)*) {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = ($($t::Generator,)*);
            #[inline]
            fn generator() -> Self::Generator {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: Generator,)*> Generator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            #[inline]
            fn generate(&mut self, _state: &mut State) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.generate(_state),)*)
            }
        }

        impl<$($t: IntoShrinker,)*> IntoShrinker for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrinker = ($($t::Shrinker,)*);
            #[inline]
            fn shrinker(self) -> Self::Shrinker {
                let ($($p,)*) = self;
                ($($p.shrinker(),)*)
            }
        }

        impl<$($t: Shrinker,)*> Shrinker for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Generator = ($($t::Generator,)*);
            #[inline]
            fn shrink(&mut self) -> Option<Self::Generator> {
                let ($($p,)*) = self;
                Some(($($p.shrink()?,)*))
            }
        }
    };
}

entia_macro::recurse_16!(tuple);
