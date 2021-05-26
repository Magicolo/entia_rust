use entia_core::bits::Bits;

use crate::{
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    item::{At, Item},
    system::Dependency,
    world::World,
};
use std::{any::TypeId, marker::PhantomData};

pub struct Query<'a, I: Item, F: Filter = ()> {
    segments: &'a Bits,
    states: &'a Vec<(I::State, usize, usize)>,
    entities: Entities<'a>,
    _marker: PhantomData<F>,
}

pub struct State<I: Item, F: Filter = ()> {
    pub(crate) index: usize,
    pub(crate) segments: Bits,
    pub(crate) states: Vec<(I::State, usize, usize)>,
    pub(crate) entities: entities::State,
    _marker: PhantomData<F>,
}

pub struct Items<'a, 'b, I: Item, F: Filter> {
    index: usize,
    segment: usize,
    query: &'b Query<'a, I, F>,
}

impl<I: Item, F: Filter> Query<'_, I, F> {
    pub fn each<E: FnMut(<I::State as At<'_>>::Item)>(&self, mut each: E) {
        for (state, _, count) in self.states {
            let count = *count;
            for i in 0..count {
                each(state.at(i));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'_>>::Item> {
        match self.entities.get_datum(entity) {
            Some(datum) => {
                let index = datum.index as usize;
                let segment = datum.segment as usize;
                for pair in self.states {
                    if pair.1 == segment {
                        return Some(pair.0.at(index));
                    }
                }
                None
            }
            None => None,
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        match self.entities.get_datum(entity) {
            Some(datum) => self.segments.has(datum.segment as usize),
            None => false,
        }
    }
}

impl<'a, 'b: 'a, I: Item, F: Filter> IntoIterator for &'b Query<'a, I, F> {
    type Item = <I::State as At<'a>>::Item;
    type IntoIter = Items<'a, 'b, I, F>;

    fn into_iter(self) -> Self::IntoIter {
        Items {
            index: 0,
            segment: 0,
            query: self,
        }
    }
}

impl<'a, 'b: 'a, I: Item, F: Filter> Iterator for Items<'a, 'b, I, F> {
    type Item = <I::State as At<'a>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, _, count)) = self.query.states.get(self.segment) {
            if self.index < *count {
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

impl<'a, I: Item + 'static, F: Filter + 'static> Inject for Query<'a, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            index: 0,
            segments: Bits::new(),
            states: Vec::new(),
            entities: state,
            _marker: PhantomData,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        for (_, segment, count) in state.states.iter_mut() {
            *count = world.segments[*segment].count;
        }

        while let Some(segment) = world.segments.get(state.index) {
            state.index += 1;

            if F::filter(segment, world) {
                if let Some(item) = I::initialize(&segment, world) {
                    state.segments.add(segment.index);
                    state.states.push((item, segment.index, segment.count));
                }
            }
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (item, segment, _) in state.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut I::depend(item, world));
        }
        dependencies
    }
}

impl<'a, I: Item + 'static, F: Filter + 'static> Get<'a> for State<I, F> {
    type Item = Query<'a, I, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            segments: &self.segments,
            states: &self.states,
            entities: self.entities.get(world),
            _marker: PhantomData,
        }
    }
}
