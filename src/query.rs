use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject, InjectContext},
    item::{At, Item, ItemContext},
    resource::Resource,
    world::World,
    write::{self, Write},
};
use std::{any::TypeId, marker::PhantomData};

pub struct Query<'a, I: Item, F: Filter = ()> {
    pub(crate) inner: &'a Inner<I, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) world: &'a World,
}

pub struct State<I: Item, F: Filter> {
    pub(crate) inner: write::State<Inner<I, F>>,
    pub(crate) entities: write::State<Entities>,
}

pub struct Items<'a, 'b, I: Item, F: Filter> {
    index: usize,
    segment: usize,
    query: &'b Query<'a, I, F>,
}

pub(crate) struct Inner<I: Item, F: Filter> {
    pub(crate) segments: Vec<Option<usize>>,
    pub(crate) states: Vec<(I::State, usize)>,
    _marker: PhantomData<fn(F)>,
}

impl<I: Item + 'static, F: Filter> Resource for Inner<I, F> {}

impl<I: Item, F: Filter> Default for Inner<I, F> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            states: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<I: Item + 'static, F: Filter> Inner<I, F> {
    pub fn segments<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.states.iter().map(|(_, segment)| *segment)
    }
}

impl<'a, I: Item, F: Filter> Query<'a, I, F> {
    pub fn len(&self) -> usize {
        self.inner
            .states
            .iter()
            .map(|&(_, segment)| self.world.segments[segment].count)
            .sum()
    }

    #[inline]
    pub fn each(&'a self, mut each: impl FnMut(<I::State as At<'a>>::Item)) {
        for (state, segment) in &self.inner.states {
            let segment = &self.world.segments[*segment];
            let count = segment.count;
            for i in 0..count {
                each(state.at(i, self.world));
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
                        return Some(state.0.at(index, self.world));
                    }
                }
                None
            }
            None => None,
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        self.entities
            .get_datum(entity)
            .and_then(|datum| self.inner.segments[datum.segment() as usize])
            .is_some()
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
        while let Some((item, segment)) = self.query.inner.states.get(self.segment) {
            let segment = &self.query.world.segments[*segment];
            if self.index < segment.count {
                let item = item.at(self.index, self.query.world);
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

unsafe impl<'a, I: Item + 'static, F: Filter> Inject for Query<'a, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, mut context: InjectContext) -> Option<Self::State> {
        let inner = <Write<Inner<I, F>> as Inject>::initialize(None, context.owned())?;
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        Some(State { inner, entities })
    }

    fn update(state: &mut Self::State, mut context: InjectContext) {
        let identifier = context.identifier();
        let world = context.world();
        let inner = state.inner.as_mut();
        while let Some(segment) = world.segments.get(inner.segments.len()) {
            if F::filter(segment, world) {
                let segment = segment.index;
                if let Some(item) = I::initialize(ItemContext::new(identifier, segment, world)) {
                    inner.segments.push(Some(segment));
                    inner.states.push((item, segment));
                    continue;
                }
            }
            inner.segments.push(None);
        }

        for (state, segment) in inner.states.iter_mut() {
            let context = ItemContext::new(context.identifier(), *segment, context.world());
            I::update(state, context);
        }
    }
}

impl<I: Item, F: Filter> Clone for State<I, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            entities: self.entities.clone(),
        }
    }
}

impl<'a, I: Item + 'static, F: Filter> Get<'a> for State<I, F> {
    type Item = Query<'a, I, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            inner: self.inner.get(world),
            entities: self.entities.get(world),
            world,
        }
    }
}

unsafe impl<I: Item + 'static, F: Filter> Depend for State<I, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.entities.depend(world);
        let inner = self.inner.as_ref();
        for (item, segment) in inner.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}
