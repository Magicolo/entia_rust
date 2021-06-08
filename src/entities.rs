use std::{
    cmp::{max, min},
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};

use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Context, Get, Inject},
    resource::Resource,
    world::World,
    write::{self, Write},
};

pub struct Entities<'a>(&'a mut Inner);
#[derive(Clone)]
pub struct State(write::State<Inner>);

#[derive(Default, Clone)]
pub struct Datum {
    index: u32,
    segment: u32,
    generation: u32,
    state: u8,
}

struct Inner {
    pub free: (Vec<Entity>, AtomicIsize),
    pub data: (Vec<Datum>, AtomicUsize),
}

impl Resource for Inner {}

impl Datum {
    const RELEASED: u8 = 0;
    const RESERVED: u8 = 1;
    const INITIALIZED: u8 = 2;

    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn segment(&self) -> u32 {
        self.segment
    }

    #[inline]
    pub fn reserve(&mut self) -> u32 {
        self.state = Self::RESERVED;
        self.generation += 1;
        self.generation
    }

    #[inline]
    pub fn release(&mut self) -> bool {
        if self.state == Self::INITIALIZED {
            self.state = Self::RELEASED;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn initialize(&mut self, index: u32, segment: u32) -> bool {
        if self.state == Self::RESERVED {
            self.index = index;
            self.segment = segment;
            self.state = Self::INITIALIZED;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn update(&mut self, index: u32, segment: u32) -> bool {
        if self.state == Self::INITIALIZED {
            self.index = index;
            self.segment = segment;
            true
        } else {
            false
        }
    }
}

impl Entities<'_> {
    #[inline]
    pub fn reserve(&mut self, entities: &mut [Entity]) {
        self.0.reserve(entities);
    }

    #[inline]
    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.0.get_datum(entity)
    }
}

impl State {
    #[inline]
    pub fn release(&mut self, entities: &[Entity]) {
        self.0.as_mut().release(entities);
    }

    #[inline]
    #[allow(dead_code)]
    pub fn get_datum(&mut self, entity: Entity) -> Option<&Datum> {
        self.0.as_ref().get_datum(entity)
    }

    #[inline]
    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.0.as_mut().get_datum_mut(entity)
    }

    #[inline]
    #[allow(dead_code)]
    pub unsafe fn get_datum_unchecked(&mut self, entity: Entity) -> &Datum {
        self.0.as_ref().get_datum_unchecked(entity)
    }

    #[inline]
    pub unsafe fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        self.0.as_mut().get_datum_mut_unchecked(entity)
    }
}

impl Inner {
    pub fn new(capacity: usize) -> Self {
        let mut free = Vec::with_capacity(capacity);
        let mut data = Vec::with_capacity(capacity);
        free.push(Entity::ZERO);
        data.push(Datum::default());
        Inner {
            free: (free, 0.into()),
            data: (data, 0.into()),
        }
    }

    pub fn reserve(&mut self, entities: &mut [Entity]) {
        if entities.len() == 0 {
            return;
        }

        let count = entities.len() as isize;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = max(min(count, last), 0) as usize;
        for i in 0..count {
            let index = last as usize - i;
            let mut entity = self.free.0[index];
            entity.generation = self.data.0[entity.index as usize].reserve();
            entities[i] = entity;
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return;
        }

        let index = self.data.1.fetch_add(remaining, Ordering::Relaxed);
        for i in 0..remaining {
            entities[i] = Entity {
                index: (index + i) as u32,
                generation: 0,
            };
        }
    }

    pub fn release(&mut self, entities: &[Entity]) {
        for entity in entities {
            self.data.0[entity.index as usize].release();
        }
        self.free.0.extend_from_slice(entities);
        *self.free.1.get_mut() = self.free.0.len() as isize;
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data.0.get(entity.index as usize).filter(|datum| {
            entity.generation == datum.generation && datum.state == Datum::INITIALIZED
        })
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data.0.get_mut(entity.index as usize).filter(|datum| {
            entity.generation == datum.generation && datum.state == Datum::INITIALIZED
        })
    }

    #[inline]
    pub unsafe fn get_datum_unchecked(&self, entity: Entity) -> &Datum {
        &self.data.0[entity.index as usize]
    }

    #[inline]
    pub unsafe fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        &mut self.data.0[entity.index as usize]
    }
}

impl Default for Inner {
    #[inline]
    fn default() -> Self {
        Self::new(32)
    }
}

impl Inject for Entities<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let inner = <Write<Inner> as Inject>::initialize(None, context, world)?;
        Some(State(inner))
    }

    fn resolve(State(state): &mut Self::State, _: &mut World) {
        let inner = state.as_mut();
        let count = max(*inner.free.1.get_mut(), 0);
        inner.free.0.truncate(count as usize);
        *inner.free.1.get_mut() = inner.free.0.len() as isize;

        let count = *inner.data.1.get_mut();
        let datum = Datum {
            index: 0,
            segment: 0,
            generation: 0,
            state: 1,
        };
        inner.data.0.resize(count, datum);
    }
}

impl<'a> Get<'a> for State {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(self.0.get(world))
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.0.depend(world)
    }
}

impl<'a> From<&'a mut State> for Entities<'a> {
    #[inline]
    fn from(state: &'a mut State) -> Self {
        Entities(state.0.as_mut())
    }
}
