use crate::{
    any::Any,
    array::{self, Array},
    collect::{self, Collect},
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    map::Map,
    primitive::{Full, Range},
    recurse,
    sample::Sample,
    shrink::Shrink,
    size::Size,
    wrap::Wrap,
};
use entia_core::Unzip;
use fastrand::Rng;
use std::iter::FromIterator;

#[derive(Clone, Debug)]
pub struct State {
    pub index: usize,
    pub count: usize,
    pub size: f64,
    pub random: Rng,
}

#[derive(Clone, Debug)]
pub struct Report<T> {
    pub state: State,
    pub original: T,
    pub shrinks: Vec<T>,
}

pub trait FullGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator() -> Self::Generate;
}

// TODO: Review all 'shrink' implementations and ensure that only one 'shrink' happens per call (ex: tuples must shrink only 1 item at a time).
// TODO: Replace 'Generator' implementations that operate directly on values (such as 'Vec<T>' and '[T; N]') with 'IntoGenerate'
// implementations?
pub trait IntoGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator(self) -> Self::Generate;
}

pub trait Generate {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink);

    fn wrap<T, B: FnMut() -> T + Clone, A: FnMut(T) + Clone>(
        self,
        before: B,
        after: A,
    ) -> Wrap<Self, T, B, A>
    where
        Self: Sized,
    {
        Wrap::generator(self, before, after)
    }

    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map::generator(self, map)
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
        FilterMap<Self, T, F>: Generate,
    {
        FilterMap::new(self, map, iterations.unwrap_or(256))
    }

    fn bind<G: Generate, F: Fn(Self::Item) -> G + Clone>(self, bind: F) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
        Flatten<Map<Self, G, F>>: Generate,
    {
        self.map(bind).flatten()
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
        Flatten<Self>: Generate,
    {
        Flatten(self)
    }

    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
        Array<Self, N>: Generate,
    {
        Array(self)
    }

    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Size<Range<usize>>, F>
    where
        Self: Sized,
        Collect<Self, Size<Range<usize>>, F>: Generate,
    {
        self.collect_with((0..256 as usize).generator())
    }

    fn collect_with<C: Generate<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
        Collect<Self, C, F>: Generate,
    {
        Collect::new(self, count)
    }

    fn size(self) -> Size<Self>
    where
        Self: Sized,
        Size<Self>: Generate,
    {
        Size(self)
    }

    fn any(self) -> Any<Self>
    where
        Self: Sized,
        Any<Self>: Generate,
    {
        Any(self)
    }

    fn sample(&self, count: usize) -> Sample<Self>
    where
        Self: Sized,
    {
        Sample::new(self, State::new(count))
    }

    fn check<F: Fn(&Self::Item) -> bool>(
        &self,
        checks: usize,
        shrinks: Option<usize>,
        check: F,
    ) -> Result<(), Report<Self::Item>>
    where
        Self: Sized,
    {
        // TODO: Parallelize checking!
        for mut state in State::new(checks) {
            let (outer_item, mut outer_shrink) = self.generate(&mut state);
            if check(&outer_item) {
                continue;
            }

            let mut report = Report {
                state,
                original: outer_item,
                shrinks: Vec::new(),
            };
            for _ in 0..shrinks.unwrap_or(usize::MAX) {
                if let Some(inner_shrink) = outer_shrink.shrink() {
                    let inner_item = inner_shrink.generate();
                    if check(&inner_item) {
                        continue;
                    }

                    report.shrinks.push(inner_item);
                    outer_shrink = inner_shrink;
                } else {
                    break;
                }
            }
            return Err(report);
        }
        Ok(())
    }
}

impl State {
    pub fn new(count: usize) -> Self {
        Self {
            index: 0,
            count,
            size: 0.,
            random: Rng::new(),
        }
    }
}

impl<T> Report<T> {
    pub fn shrunk(&self) -> &T {
        self.shrinks.last().unwrap_or(&self.original)
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

impl<G: Generate> Generate for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        (*self).generate(state)
    }
}

impl<G: FullGenerate, const N: usize> FullGenerate for [G; N] {
    type Item = [G::Item; N];
    type Generate = [G::Generate; N];
    fn generator() -> Self::Generate {
        [(); N].map(|_| G::generator())
    }
}

impl<G: IntoGenerate, const N: usize> IntoGenerate for [G; N] {
    type Item = [G::Item; N];
    type Generate = [G::Generate; N];
    fn generator(self) -> Self::Generate {
        self.map(|generate| generate.generator())
    }
}

impl<G: Generate, const N: usize> Generate for [G; N] {
    type Item = [G::Item; N];
    type Shrink = array::Shrinker<G::Shrink, N>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let mut index = 0;
        let (items, shrinks) = [(); N]
            .map(|_| {
                let pair = self[index].generate(state);
                index += 1;
                pair
            })
            .unzip();
        (items, array::Shrinker(shrinks))
    }
}

impl<G: FullGenerate> FullGenerate for Vec<G> {
    type Item = Vec<G::Item>;
    type Generate = Collect<G::Generate, Size<Range<usize>>, Self::Item>;
    fn generator() -> Self::Generate {
        G::generator().collect()
    }
}

impl<G: IntoGenerate> IntoGenerate for Vec<G> {
    type Item = Vec<G::Item>;
    type Generate = Vec<G::Generate>;
    fn generator(self) -> Self::Generate {
        self.into_iter()
            .map(|generate| generate.generator())
            .collect()
    }
}

impl<G: Generate> Generate for Vec<G> {
    type Item = Vec<G::Item>;
    type Shrink = collect::Shrinker<G::Shrink, Vec<G::Item>>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (items, shrinks) = self.iter().map(|generate| generate.generate(state)).unzip();
        (items, collect::Shrinker::new(shrinks))
    }
}

impl FullGenerate for String {
    type Item = Self;
    type Generate = Collect<Size<Full<char>>, Size<Range<usize>>, Self::Item>;
    fn generator() -> Self::Generate {
        char::generator().collect()
    }
}

macro_rules! tuple {
    () => {
        impl FullGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator() -> Self::Generate { () }
        }

        impl IntoGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator(self) -> Self::Generate { self }
        }

        impl Generate for () {
            type Item = ();
            type Shrink = ();
            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) { ((), ()) }
        }

        impl Shrink for () {
            type Item = ();
            fn generate(&self) -> Self::Item { () }
            fn shrink(&mut self) -> Option<Self> { None }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t: FullGenerate,)*> FullGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator() -> Self::Generate {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: IntoGenerate,)*> IntoGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator(self) -> Self::Generate {
                let ($($p,)*) = self;
                ($($p.generator(),)*)
            }
        }

        impl<$($t: Generate,)*> Generate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) {
                let ($($p,)*) = self;
                $(let $p = $p.generate(_state);)*
                (($($p.0,)*), ($($p.1,)*))
            }
        }

        impl<$($t: Shrink,)*> Shrink for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn generate(&self) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.generate(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                let ($($p,)*) = self;
                let mut shrunk = false;
                let ($($p,)*) = ($(
                    if shrunk { $p.clone() }
                    else {
                        match $p.shrink() {
                            Some(shrink) => { shrunk = true; shrink },
                            None => $p.clone(),
                        }
                    },
                )*);
                if shrunk { Some(($($p,)*)) } else { None }
            }
        }
    };
}

recurse!(tuple);
