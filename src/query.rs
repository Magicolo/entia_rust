use crate::inject::*;
use crate::item::*;
use crate::system::*;
use crate::world::*;
use std::sync::Arc;

pub struct Query<'a, I: Item> {
    states: &'a Vec<(I::State, Arc<Segment>)>,
}

pub struct QueryState<I: Item>(usize, Vec<(I::State, Arc<Segment>)>);

pub struct QueryIterator<'a, I: Item> {
    index: usize,
    segment: usize,
    query: Query<'a, I>,
}

impl<'a, I: Item> Query<'a, I> {
    #[inline]
    pub fn each<F: FnMut(<I::State as At>::Item)>(&self, mut each: F) {
        for (item, segment) in self.states.iter() {
            for i in 0..segment.count {
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
            query: self.clone(),
        }
    }
}

impl<'a, I: Item> Iterator for QueryIterator<'a, I> {
    type Item = <I::State as At<'a>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, segment)) = self.query.states.get(self.segment) {
            if self.index < segment.count {
                let item = item.at(self.index);
                self.index += 1;
                return Some(item);
            } else {
                self.segment += 1;
                self.index = 0;
            }
        }
        None
    }
}

impl<'a, I: Item + 'static> Inject for Query<'a, I> {
    type State = QueryState<I>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(QueryState(0, Vec::new()))
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.0) {
            if let Some(item) = I::initialize(&segment) {
                state.1.push((item, segment.clone()));
            }
            state.0 += 1;
        }
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (item, _) in state.1.iter() {
            dependencies.append(&mut I::dependencies(item));
        }
        dependencies
    }
}

impl<'a, I: Item> Get<'a> for QueryState<I> {
    type Item = Query<'a, I>;

    fn get(&'a mut self, _: &World) -> Self::Item {
        Query { states: &self.1 }
    }
}
