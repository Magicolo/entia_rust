use crate::entity::*;
use crate::inject::*;
use crate::item::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Query<'a, I: Item> {
    states: &'a Vec<(I::State, Arc<Segment>)>,
}

pub struct QueryState<I: Item>(usize, Vec<(I::State, Arc<Segment>)>, Filter);

pub struct QueryIterator<'a, I: Item> {
    index: usize,
    count: usize,
    segment: usize,
    query: Query<'a, I>,
}

pub struct Filter(fn(&Segment) -> bool);

impl Default for Filter {
    fn default() -> Self {
        Self::with::<Entity>()
    }
}

impl Filter {
    pub const TRUE: Self = Self(|_| true);
    pub const FALSE: Self = Self(|_| false);

    #[inline]
    pub fn new(filter: fn(&Segment) -> bool) -> Self {
        Self(filter)
    }

    #[inline]
    pub fn with<T: Send + 'static>() -> Self {
        Self::new(|segment| segment.store::<T>().is_some())
    }

    #[inline]
    pub fn filter(&self, segment: &Segment) -> bool {
        self.0(segment)
    }
}

impl<'a, I: Item> Query<'a, I> {
    pub fn each<F: FnMut(<I::State as At<'a>>::Item)>(&self, mut each: F) {
        for (item, segment) in self.states.iter() {
            let count = segment.count.load(Ordering::Relaxed);
            for i in 0..count {
                each(item.at(i));
            }
        }
    }
}

impl<'a, I: Item> Clone for Query<'a, I> {
    fn clone(&self) -> Self {
        Query {
            states: self.states.clone(),
        }
    }
}

impl<'a, I: Item> IntoIterator for Query<'a, I> {
    type Item = <I::State as At<'a>>::Item;
    type IntoIter = QueryIterator<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIterator {
            segment: 0,
            index: 0,
            count: 0,
            query: self,
        }
    }
}

impl<'a, I: Item> IntoIterator for &Query<'a, I> {
    type Item = <I::State as At<'a>>::Item;
    type IntoIter = QueryIterator<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIterator {
            segment: 0,
            index: 0,
            count: 0,
            query: self.clone(),
        }
    }
}

impl<'a, I: Item> Iterator for QueryIterator<'a, I> {
    type Item = <I::State as At<'a>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, segment)) = self.query.states.get(self.segment) {
            if self.count == 0 {
                self.count = segment.count.load(Ordering::Relaxed);
            }

            if self.index < self.count {
                let item = item.at(self.index);
                self.index += 1;
                return Some(item);
            } else {
                self.segment += 1;
                self.index = 0;
                self.count = 0;
            }
        }
        None
    }
}

impl<'a, I: Item + 'static> Inject for Query<'a, I> {
    type Input = Filter;
    type State = QueryState<I>;

    fn initialize(input: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(QueryState(0, Vec::new(), input))
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.0) {
            state.0 += 1;

            if state.2.filter(segment) {
                if let Some(item) = I::initialize(&segment) {
                    state.1.push((item, segment.clone()));
                }
            }
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (item, segment) in state.1.iter() {
            dependencies.push(Dependency::Read(segment.index, TypeId::of::<Entity>()));
            dependencies.append(&mut I::depend(item, world));
        }
        dependencies
    }
}

impl<'a, I: Item + 'static> Get<'a> for QueryState<I> {
    type Item = Query<'a, I>;

    fn get(&'a mut self, _: &World) -> Self::Item {
        Query { states: &self.1 }
    }
}
