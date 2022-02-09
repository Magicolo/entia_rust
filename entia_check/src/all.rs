use crate::{
    generator::{FullGenerator, Generator, State},
    recurse,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct All<G>(pub G);

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

    fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
        let (items, states) = generate_array(|index| self.0[index].generate(state));
        (items, (states, 0))
    }

    fn shrink(&self, state: &mut Self::State) -> Option<Self> {
        shrink_array(&self.0, state).map(All)
    }
}

pub(crate) fn generate_array<T, S, F: FnMut(usize) -> (T, S), const N: usize>(
    mut map: F,
) -> ([T; N], [S; N]) {
    let mut items = [(); N].map(|_| None);
    let mut states = [(); N].map(|_| None);
    for i in 0..N {
        let (item, state) = map(i);
        items[i] = Some(item);
        states[i] = Some(state);
    }
    (items.map(Option::unwrap), states.map(Option::unwrap))
}

pub(crate) fn shrink_array<G: Generator, const N: usize>(
    array: &[G; N],
    state: &mut ([G::State; N], usize),
) -> Option<[G; N]> {
    while state.1 < N {
        if let Some(shrink) = array[state.1].shrink(&mut state.0[state.1]) {
            let mut generators = array.clone();
            generators[state.1] = shrink;
            return Some(generators);
        } else {
            state.1 += 1;
        }
    }
    None
}

impl<G: Generator> From<Vec<G>> for All<Vec<G>> {
    fn from(generators: Vec<G>) -> Self {
        Self(generators)
    }
}

impl<G: Generator> Generator for All<Vec<G>> {
    type Item = Vec<G::Item>;
    type State = (Vec<G::State>, usize, usize);

    fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
        let (items, states) = self
            .0
            .iter()
            .map(|generator| generator.generate(state))
            .unzip();
        (items, (states, 0, 0))
    }

    fn shrink(&self, state: &mut Self::State) -> Option<Self> {
        shrink_vector(&self.0, state).map(All)
    }
}

pub(crate) fn shrink_vector<G: Generator>(
    vector: &Vec<G>,
    state: &mut (Vec<G::State>, usize, usize),
) -> Option<Vec<G>> {
    // Try to remove irrelevant generators.
    if state.1 < vector.len() {
        let mut generators = vector.clone();
        generators.remove(state.1);
        state.1 += 1;
        return Some(generators);
    }

    // Try to shrink each item and succeed if any item is shrunk.
    while state.2 < vector.len() {
        if let Some(shrink) = vector[state.2].shrink(&mut state.0[state.2]) {
            let mut generators = vector.clone();
            generators[state.2] = shrink;
            return Some(generators);
        } else {
            state.2 += 1;
        }
    }

    None
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
            fn generate(&self, _state: &mut State) -> (Self::Item, Self::State) { ((), ()) }
            fn shrink(&self, _state: &mut Self::State) -> Option<Self> { None }
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

            fn generate(&self, _state: &mut State) -> (Self::Item, Self::State) {
                let ($($p,)*) = self;
                $(let $p = $p.generate(_state);)*
                (($($p.0,)*), ($($p.1,)*))
            }

            fn shrink(&self, ($($t,)*): &mut Self::State) -> Option<Self> {
                let ($($p,)*) = self;
                // TODO: Shrink one element at a time. 'Shrink' type will be '(Or<$t, $t::Shrink>, ...)'.
                Some(($($p.shrink($t)?,)*))
            }
        }
    };
}

recurse!(tuple);
