use crate::entity::*;
use crate::inject::*;
use crate::item::*;
use crate::segment::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;

pub struct Query<'a, I: Item> {
    states: &'a Vec<(I::State, usize)>,
    world: &'a World,
}

pub struct QueryState<I: Item> {
    index: usize,
    states: Vec<(I::State, usize)>,
    filter: Filter,
}

pub struct QueryIterator<'a, I: Item> {
    index: usize,
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
        Self::new(|segment| segment.static_store::<T>().is_some())
    }

    #[inline]
    pub fn filter(&self, segment: &Segment) -> bool {
        self.0(segment)
    }
}

impl<'a, I: Item> Query<'a, I> {
    pub fn each<F: FnMut(<I::State as At<'a>>::Item)>(&self, mut each: F) {
        for (item, segment) in self.states.iter() {
            let segment = &self.world.segments[*segment];
            for i in 0..segment.count {
                each(item.at(i));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'a>>::Item> {}
}

impl<'a, I: Item> Clone for Query<'a, I> {
    fn clone(&self) -> Self {
        Query {
            states: self.states,
            world: self.world,
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
            let segment = &self.query.world.segments[*segment];
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
    type Input = Filter;
    type State = QueryState<I>;

    fn initialize(input: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(QueryState {
            index: 0,
            states: Vec::new(),
            filter: input,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.index) {
            state.index += 1;

            if state.filter.filter(segment) {
                if let Some(item) = I::initialize(&segment) {
                    state.states.push((item, segment.index));
                }
            }
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (item, segment) in state.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut I::depend(item, world));
        }
        dependencies
    }
}

impl<'a, I: Item + 'static> Get<'a> for QueryState<I> {
    type Item = Query<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            states: &self.states,
            world,
        }
    }
}
