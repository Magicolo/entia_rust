use std::{any::TypeId, array::IntoIter, cmp::min};

use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    initialize::Initialize,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
};

pub struct Create<'a, I: Initialize>(Defer<'a, Creation<I>>, &'a World);
pub struct State<I: Initialize>(defer::State<Creation<I>>);

struct Creation<I>(Vec<Entity>, Vec<I>, usize);

/*
TODO:
- Queries must store the segment count at update time.
- Create and Destroy should be exclusive if they overlap. They might not strictly need to be exclusive, but lets assume it for now.
- As long as a Create operation does not resize a segment, it can be resolved at run time, otherwise it is deferred.

*/

impl<I: Initialize> Create<'_, I> {
    pub fn one(&mut self, initialize: I) -> Entity {
        self.many(IntoIter::new([initialize]))[0]
    }

    pub fn many(&mut self, mut intializes: impl ExactSizeIterator<Item = I>) -> &[Entity] {
        let count = intializes.len();
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
                let initialize = intializes.next().unwrap();
                let entity = buffer[i];
                let index = pair.0 + i;
                initialize.set(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { segment.stores[0].set_all(pair.0, &buffer[..ready]) };
        }

        let defer = Creation(buffer, intializes.collect(), pair.0 + ready);
        &self.0.defer(defer).0
    }

    #[inline]
    pub fn many_clone(&mut self, initialize: I, count: usize) -> &[Entity]
    where
        I: Clone,
    {
        self.many((0..count).map(move |_| initialize.clone()))
    }

    #[inline]
    pub fn many_default(&mut self, count: usize) -> &[Entity]
    where
        I: Default,
    {
        self.many((0..count).map(|_| I::default()))
    }
}

impl<I: Initialize> Inject for Create<'_, I> {
    type Input = ();
    type State = State<I>;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let meta = world.get_or_add_meta::<Entity>();
        let mut metas = I::metas(world);
        metas.push(meta);

        let segment = world.get_or_add_segment_by_metas(metas).index;
        let state = I::initialize(&world.segments[segment], world)?;
        let entities = <Write<Entities> as Inject>::initialize(None, context, world)?;
        let input = (entities, state, segment);
        let defer = <Defer<Creation<I>> as Inject>::initialize(input, context, world)?;
        Some(State(defer))
    }

    fn update(State(state): &mut Self::State, world: &mut World) {
        <Defer<Creation<I>> as Inject>::update(state, world);
    }

    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Defer<Creation<I>> as Inject>::resolve(state, world);
    }
}

impl<I: Initialize> Clone for State<I> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, I: Initialize> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create(self.0.get(world), world)
    }
}

unsafe impl<I: Initialize> Depend for State<I> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let state = self.0.as_ref();
        dependencies.push(Dependency::Defer(state.2, TypeId::of::<Entity>()));
        dependencies
    }
}

impl<I: Initialize> Resolve for Creation<I> {
    type State = (write::State<Entities>, I::State, usize);

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
            for (i, initialize) in item.1.drain(..).enumerate() {
                let index = item.2 + i;
                let entity = item.0[offset + i];
                initialize.set(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { store.set_all(item.2, &item.0[offset..]) };
        }
    }
}
