use std::{any::TypeId, array::IntoIter, cmp::min, convert::TryInto};

use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    initial::Initial,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
};

pub struct Create<'a, I: Initial>(Defer<'a, Creation<I>>, &'a World);
pub struct State<I: Initial>(defer::State<Creation<I>>);

struct Creation<I>(Vec<Entity>, Vec<I>, usize);

impl<I: Initial> Create<'_, I> {
    pub fn all(&mut self, mut initials: impl ExactSizeIterator<Item = I>) -> &[Entity] {
        let count = initials.len();
        if count == 0 {
            return &[];
        }

        // TODO: Try to prevent 'buffer' heap allocation when the size is statically known. Should be possible to only
        // move to the heap when a defferal is needed.
        let state = self.0.state();
        let entities = state.0.as_mut();
        let mut buffer = vec![Entity::ZERO; count];
        let valid = entities.reserve(&mut buffer);
        let segment = &self.1.segments[state.2];
        let pair = segment.reserve(count);
        let ready = min(valid, pair.1);
        if ready > 0 {
            for i in 0..ready {
                let initialize = initials.next().unwrap();
                let entity = buffer[i];
                let index = pair.0 + i;
                initialize.apply(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { segment.stores[0].set_all(pair.0, &buffer[..ready]) };
        }

        let defer = Creation(buffer, initials.collect(), pair.0 + ready);
        &self.0.defer(defer).0
    }

    #[inline]
    pub fn one(&mut self, initial: I) -> Entity {
        self.all(IntoIter::new([initial]))[0]
    }

    #[inline]
    pub fn exact<const N: usize>(&mut self, initials: [I; N]) -> &[Entity; N] {
        self.all(IntoIter::new(initials)).try_into().unwrap()
    }

    #[inline]
    pub fn clones(&mut self, initial: I, count: usize) -> &[Entity]
    where
        I: Clone,
    {
        self.all((0..count).map(move |_| initial.clone()))
    }

    #[inline]
    pub fn defaults(&mut self, count: usize) -> &[Entity]
    where
        I: Default,
    {
        self.all((0..count).map(|_| I::default()))
    }
}

impl<I: Initial> Inject for Create<'_, I> {
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

impl<I: Initial> Clone for State<I> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, I: Initial> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create(self.0.get(world), world)
    }
}

unsafe impl<I: Initial> Depend for State<I> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let state = self.0.as_ref();
        dependencies.push(Dependency::Defer(state.2, TypeId::of::<Entity>()));
        dependencies
    }
}

impl<I: Initial> Resolve for Creation<I> {
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
                initialize.apply(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
            unsafe { store.set_all(item.2, &item.0[offset..]) };
        }
    }
}
