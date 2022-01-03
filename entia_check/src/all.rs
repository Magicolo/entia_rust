use crate::generator::{FullGenerator, Generator, State};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct All<T>(T);

impl<G: FullGenerator, const N: usize> FullGenerator for [G; N] {
    type Item = [G::Item; N];
    type Generator = All<[G::Generator; N]>;
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
    type Shrink = All<[G::Shrink; N]>;

    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let mut index = 0;
        [(); N].map(|_| {
            let item = self.0[index].generate(state);
            index += 1;
            item
        })
    }

    fn shrink(&mut self) -> Option<Self::Shrink> {
        // TODO: Is there a way to abort shrinking as soon as a 'None' is produced?
        // let generators = [None; N];
        // for i in 0..N {
        //     generators[i] = Some(self.0[i].shrink()?);
        // }

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
    type Shrink = All<Vec<G::Shrink>>;

    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let map = |generator: &mut G| generator.generate(state);
        self.0.iter_mut().map(map).collect()
    }

    fn shrink(&mut self) -> Option<Self::Shrink> {
        // TODO: Try to remove irrelevant generators...
        let mut generators = Vec::with_capacity(self.0.len());
        for shrinker in &mut self.0 {
            generators.push(shrinker.shrink()?);
        }
        Some(All(generators))
    }
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: FullGenerator,)*> FullGenerator for ($($t,)*) {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = ($($t::Generator,)*);
            #[inline]
            fn generator() -> Self::Generator {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: Generator,)*> Generator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            #[inline]
            fn generate(&mut self, _state: &mut State) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.generate(_state),)*)
            }

            fn shrink(&mut self) -> Option<Self::Shrink> {
                let ($($p,)*) = self;
                Some(($($p.shrink()?,)*))
            }
        }
    };
}

entia_macro::recurse_16!(tuple);
