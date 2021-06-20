use std::{any::TypeId, array::IntoIter, cmp::min};

use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Context, Get, Inject},
    set::Set,
    world::World,
    write::{self, Write},
};

pub struct Create<'a, S: Set>(Defer<'a, Creation<S>>, &'a World);
pub struct State<S: Set>(defer::State<Creation<S>>);

struct Creation<S>(Vec<Entity>, Vec<S>, usize);

/*
TODO:
- Queries must store the segment count at update time.
- Create and Destroy should be exclusive if they overlap. They might not strictly need to be exclusive, but lets assume it for now.
- As long as a Create operation does not resize a segment, it can be resolved at run time, otherwise it is deferred.

*/

impl<S: Set> Create<'_, S> {
    pub fn one(&mut self, set: S) -> Entity {
        self.many(IntoIter::new([set]))[0]
    }

    pub fn many(&mut self, mut sets: impl ExactSizeIterator<Item = S>) -> &[Entity] {
        let count = sets.len();
        if count == 0 {
            return &[];
        }

        let state = self.0.state();
        let entities = state.0.as_mut();
        let mut buffer = vec![Entity::ZERO; count];
        let valid = entities.reserve(&mut buffer);
        let segment = &self.1.segments[state.2];
        let pair = segment.prepare(count);
        let ready = min(valid, pair.1);
        if ready > 0 {
            for i in 0..ready {
                let set = sets.next().unwrap();
                let entity = buffer[i];
                let index = pair.0 + i;
                set.set(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { segment.stores[0].set_all(pair.0, &buffer[..ready]) };
        }

        let defer = Creation(buffer, sets.collect(), pair.0 + ready);
        &self.0.defer(defer).0
    }

    #[inline]
    pub fn many_clone(&mut self, set: S, count: usize) -> &[Entity]
    where
        S: Clone,
    {
        self.many((0..count).map(move |_| set.clone()))
    }

    #[inline]
    pub fn many_default(&mut self, count: usize) -> &[Entity]
    where
        S: Default,
    {
        self.many((0..count).map(|_| S::default()))
    }
}

impl<S: Set> Inject for Create<'_, S> {
    type Input = ();
    type State = State<S>;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let meta = world.get_or_add_meta::<Entity>();
        let mut metas = S::metas(world);
        metas.push(meta);

        let segment = world.get_or_add_segment_by_metas(metas).index;
        let state = S::initialize(&world.segments[segment], world)?;
        let entities = <Write<Entities> as Inject>::initialize(None, context, world)?;
        let input = (entities, state, segment);
        let defer = <Defer<Creation<S>> as Inject>::initialize(input, context, world)?;
        Some(State(defer))
    }

    fn update(State(state): &mut Self::State, world: &mut World) {
        <Defer<Creation<S>> as Inject>::update(state, world);
    }

    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Defer<Creation<S>> as Inject>::resolve(state, world);
    }
}

impl<S: Set> Clone for State<S> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, S: Set> Get<'a> for State<S> {
    type Item = Create<'a, S>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create(self.0.get(world), world)
    }
}

unsafe impl<S: Set> Depend for State<S> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let state = self.0.as_ref();
        dependencies.push(Dependency::Defer(state.2, TypeId::of::<Entity>()));
        dependencies
    }
}

impl<S: Set> Resolve for Creation<S> {
    type State = (write::State<Entities>, S::State, usize);

    fn resolve(items: impl Iterator<Item = Self>, state: &mut Self::State, world: &mut World) {
        let entities = state.0.as_mut();
        let segment = &mut world.segments[state.2];
        let store = segment.stores[0].clone();
        entities.resolve();
        segment.resolve();

        for mut item in items {
            if item.0.len() == 0 || item.1.len() == 0 {
                continue;
            }

            // The entities can be assumed to have not been destroyed since this operation has been enqueued before any other
            // destroy operation that could concern them.
            let offset = item.0.len() - item.1.len();
            for (i, set) in item.1.drain(..).enumerate() {
                let index = item.2 + i;
                let entity = item.0[offset + i];
                set.set(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { store.set_all(item.2, &item.0[offset..]) };
        }
    }
}
