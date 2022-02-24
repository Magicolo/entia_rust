use crate::{
    sample::Sample,
    array::Array,
    collect::Collect,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    map::Map,
    primitive::{Full, Range},
    shrink::Shrink,
    size::Size,
    wrap::Wrap,
};
use fastrand::Rng;
use std::iter::FromIterator;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub index: usize,
    pub count: usize,
    pub size: f64,
    pub random: Rng,
}

#[derive(Debug)]
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

pub trait Generate: Sized + Clone {
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
    {
        FilterMap::new(self, map, iterations.unwrap_or(256))
    }

    fn bind<G: Generate, F: Fn(Self::Item) -> G + Clone>(self, bind: F) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
    {
        self.map(bind).flatten()
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
    {
        Flatten::new(self)
    }

    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        Array(self)
    }

    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Size<Range<usize>>, F>
    where
        Self: Sized,
    {
        self.collect_with((0..256 as usize).generator())
    }

    fn collect_with<C: Generate<Item = usize>, F: FromIterator<Self::Item>>(
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
        Size<Self>: Generate,
    {
        Size::from(self)
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
    ) -> Result<(), Report<Self::Item>> {
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

impl FullGenerate for String {
    type Item = Self;
    type Generate = Collect<Size<Full<char>>, Size<Range<usize>>, Self::Item>;
    fn generator() -> Self::Generate {
        char::generator().collect()
    }
}

impl<G: FullGenerate> FullGenerate for Vec<G> {
    type Item = Vec<G::Item>;
    type Generate = Collect<G::Generate, Size<Range<usize>>, Self::Item>;
    fn generator() -> Self::Generate {
        G::generator().collect()
    }
}
