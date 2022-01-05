use crate::generator::{or::Or, FullGenerator, Generator, State};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct All<T>(T);

impl<G: FullGenerator, const N: usize> FullGenerator for [G; N] {
    type Item = [G::Item; N];
    type Generator = All<[G::Generator; N]>;
    fn generator() -> Self::Generator {
        All::from([(); N].map(|_| G::generator()))
    }
}

impl<G: Generator, const N: usize> From<[G; N]> for All<[G; N]> {
    fn from(generators: [G; N]) -> Self {
        Self(generators)
    }
}

impl<G: Generator, const N: usize> Generator for All<[G; N]> {
    type Item = [G::Item; N];
    type State = ([G::State; N], usize);
    type Shrink = All<[Or<G, G::Shrink>; N]>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
        let mut items = [(); N].map(|_| None);
        let mut states = [(); N].map(|_| None);
        for i in 0..N {
            let (item, state) = self.0[i].generate(state);
            items[i] = Some(item);
            states[i] = Some(state);
        }
        (items.map(Option::unwrap), (states.map(Option::unwrap), 0))
    }

    fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
        while state.1 < N {
            if let Some(shrink) = self.0[state.1].shrink(&mut state.0[state.1]) {
                let mut generators = self.0.clone().map(Or::Left);
                generators[state.1] = Or::Right(shrink);
                return Some(All(generators));
            } else {
                state.1 += 1;
            }
        }

        None
    }
}

impl<G: Generator> From<Vec<G>> for All<Vec<G>> {
    fn from(generators: Vec<G>) -> Self {
        Self(generators)
    }
}

impl<G: Generator> Generator for All<Vec<G>> {
    type Item = Vec<G::Item>;
    type State = (Vec<G::State>, usize, usize);
    type Shrink = All<Vec<Or<G, G::Shrink>>>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
        let (items, states) = self
            .0
            .iter()
            .map(|generator| generator.generate(state))
            .unzip();
        (items, (states, 0, 0))
    }

    fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
        // Try to remove irrelevant items.
        if state.1 < self.0.len() {
            let mut generators = self.0.iter().cloned().map(Or::Left).collect::<Vec<_>>();
            generators.remove(state.1);
            state.1 += 1;
            return Some(All(generators));
        }

        // Try to shrink each item and succeed if any item is shrunk.
        while state.2 < self.0.len() {
            if let Some(shrink) = self.0[state.2].shrink(&mut state.0[state.2]) {
                let mut generators = self.0.iter().cloned().map(Or::Left).collect::<Vec<_>>();
                generators[state.2] = Or::Right(shrink);
                return Some(All(generators));
            } else {
                state.2 += 1;
            }
        }

        None
    }
}

macro_rules! tuple {
    () => {
        impl FullGenerator for () {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = ();
            fn generator() -> Self::Generator { () }
        }

        impl Generator for () {
            type Item = ();
            type State = ();
            type Shrink = ();
            fn generate(&self, _state: &mut State) -> (Self::Item, Self::State) { ((), ()) }
            fn shrink(&self, _state: &mut Self::State) -> Option<Self::Shrink> { None }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t: FullGenerator,)*> FullGenerator for ($($t,)*) {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = ($($t::Generator,)*);
            fn generator() -> Self::Generator {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: Generator,)*> Generator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type State = ($($t::State,)*);
            type Shrink = ($($t::Shrink,)*);

            fn generate(&self, _state: &mut State) -> (Self::Item, Self::State) {
                let ($($p,)*) = self;
                $(let $p = $p.generate(_state);)*
                (($($p.0,)*), ($($p.1,)*))
            }

            fn shrink(&self, ($($t,)*): &mut Self::State) -> Option<Self::Shrink> {
                let ($($p,)*) = self;
                Some(($($p.shrink($t)?,)*))
            }
        }
    };
}

entia_macro::recurse_16!(tuple);
