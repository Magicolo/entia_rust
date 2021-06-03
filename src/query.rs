use entia_core::bits::Bits;

use crate::{
    depend::{Depend, Dependency},
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    item::{At, Item},
    resource::Resource,
    world::World,
    write::{self, Write},
};
use std::{any::TypeId, marker::PhantomData};

pub struct Query<'a, I: Item, F: Filter = ()> {
    inner: &'a mut Inner<I, F>,
    entities: Entities<'a>,
}

pub struct State<I: Item, F: Filter> {
    pub(crate) inner: write::State<Inner<I, F>>,
    pub(crate) entities: entities::State,
}

pub struct Items<'a, 'b, I: Item, F: Filter> {
    index: usize,
    segment: usize,
    query: &'b Query<'a, I, F>,
}

pub(crate) struct Inner<I: Item, F: Filter> {
    pub(crate) index: usize,
    pub(crate) segments: Bits,
    pub(crate) states: Vec<(I::State, usize, usize)>,
    _marker: PhantomData<F>,
}

impl<I: Item + 'static, F: Filter> Resource for Inner<I, F> {}

impl<I: Item, F: Filter> Default for Inner<I, F> {
    fn default() -> Self {
        Self {
            index: 0,
            segments: Bits::new(),
            states: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a, I: Item, F: Filter> Query<'a, I, F> {
    #[inline]
    pub fn each(&'a self, mut each: impl FnMut(<I::State as At<'a>>::Item)) {
        for (state, _, count) in &self.inner.states {
            let count = *count;
            for i in 0..count {
                each(state.at(i));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'_>>::Item> {
        match self.entities.get_datum(entity) {
            Some(datum) => {
                let index = datum.index() as usize;
                let segment = datum.segment() as usize;
                for state in &self.inner.states {
                    if state.1 == segment {
                        return Some(state.0.at(index));
                    }
                }
                None
            }
            None => None,
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        match self.entities.get_datum(entity) {
            Some(datum) => self.inner.segments.has(datum.segment() as usize),
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
        while let Some((item, _, count)) = self.query.inner.states.get(self.segment) {
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

impl<'a, I: Item + 'static, F: Filter> Inject for Query<'a, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let inner = <Write<Inner<I, F>> as Inject>::initialize(None, world)?;
        let entities = <Entities as Inject>::initialize((), world)?;
        Some(State { inner, entities })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        let inner = state.inner.as_mut();
        for (_, segment, count) in inner.states.iter_mut() {
            *count = world.segments[*segment].count;
        }

        while let Some(segment) = world.segments.get(inner.index) {
            inner.index += 1;

            if F::filter(segment, world) {
                if let Some(item) = I::initialize(&segment, world) {
                    inner.segments.set(segment.index, true);
                    inner.states.push((item, segment.index, segment.count));
                }
            }
        }
    }
}

impl<'a, I: Item + 'static, F: Filter> Get<'a> for State<I, F> {
    type Item = Query<'a, I, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            inner: self.inner.get(world),
            entities: self.entities.get(world),
        }
    }
}

impl<I: Item + 'static, F: Filter> Depend for State<I, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        let inner = self.inner.as_ref();
        for (item, segment, _) in inner.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}
