use std::{any::TypeId, array::IntoIter, cmp::min, convert::TryInto, mem::replace};

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

pub struct Create<'a, I: Initial> {
    defer: Defer<'a, Creation<I>>,
    world: &'a World,
    entities: &'a mut Vec<Entity>,
}
pub struct State<I: Initial> {
    defer: defer::State<Creation<I>>,
    entities: Vec<Entity>,
}
struct Creation<I> {
    entities: Vec<Entity>,
    initials: Vec<I>,
    index: usize,
    ready: usize,
}

impl<I: Initial> Create<'_, I> {
    pub fn all(&mut self, mut initials: impl ExactSizeIterator<Item = I>) -> &[Entity] {
        let count = initials.len();
        if count == 0 {
            return &[];
        }

        let state = self.defer.state();
        let entities = state.0.as_mut();
        let segment = &self.world.segments[state.2];

        self.entities.resize(count, Entity::ZERO);
        let valid = entities.reserve(self.entities);
        let pair = segment.reserve(count);
        let ready = min(valid, pair.1);

        if ready > 0 {
            unsafe { segment.stores[0].set_all(pair.0, &self.entities[..ready]) };
            for i in 0..ready {
                let initialize = initials.next().unwrap();
                let entity = self.entities[i];
                let index = pair.0 + i;
                initialize.apply(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
        }

        if ready < count {
            let entities = replace(self.entities, Vec::new());
            let defer = Creation {
                entities,
                initials: initials.collect(),
                index: pair.0,
                ready,
            };
            &self.defer.defer(defer).entities
        } else {
            &self.entities
        }
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
        Some(State {
            defer,
            entities: Vec::new(),
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Defer<Creation<I>> as Inject>::update(&mut state.defer, world);
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let (entities, _, segment) = state.defer.as_mut();
        entities.as_mut().resolve();
        world.segments[*segment].resolve();
        <Defer<Creation<I>> as Inject>::resolve(&mut state.defer, world);
    }
}

impl<I: Initial> Clone for State<I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            defer: self.defer.clone(),
            entities: self.entities.clone(),
        }
    }
}

impl<'a, I: Initial> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            defer: self.defer.get(world),
            world,
            entities: &mut self.entities,
        }
    }
}

unsafe impl<I: Initial> Depend for State<I> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.defer.depend(world);
        let state = self.defer.as_ref();
        dependencies.push(Dependency::Defer(state.2, TypeId::of::<Entity>()));
        dependencies
    }
}

impl<I: Initial> Resolve for Creation<I> {
    type State = (write::State<Entities>, I::State, usize);

    fn resolve(items: impl Iterator<Item = Self>, state: &mut Self::State, world: &mut World) {
        let entities = state.0.as_mut();
        let segment = &mut world.segments[state.2];

        for mut item in items {
            if item.entities.len() == 0 || item.initials.len() == 0 {
                continue;
            }

            let index = item.index + item.ready;
            unsafe { segment.stores[0].set_all(index, &item.entities[item.ready..]) };
            for (i, initialize) in item.initials.drain(..).enumerate() {
                let index = index + i;
                let entity = item.entities[item.ready + i];
                initialize.apply(&mut state.1, index);
                entities
                    .get_datum_mut_unchecked(entity)
                    .initialize(index as u32, segment.index as u32);
            }
        }
    }
}
