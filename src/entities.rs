use std::{
    cmp::{max, min},
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};

use crate::{entity::Entity, resource::Resource};

pub struct Entities {
    free: (Vec<Entity>, AtomicIsize),
    data: (Vec<Datum>, AtomicUsize),
}

#[derive(Default, Clone)]
pub struct Datum {
    store: u32,
    segment: u32,
    generation: u32,
    parent: u32,
    child: u32,
    sibling: u32,
    state: State,
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum State {
    Released = 0,
    Initialized = 1,
}

impl Resource for Entities {}

impl Default for State {
    fn default() -> Self {
        Self::Released
    }
}

impl Datum {
    #[inline]
    pub const fn index(&self) -> u32 {
        self.store
    }

    #[inline]
    pub const fn segment(&self) -> u32 {
        self.segment
    }

    #[inline]
    pub fn release(&mut self) -> bool {
        if matches!(self.state, State::Initialized) {
            self.state = State::Released;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn initialize(
        &mut self,
        generation: u32,
        store: u32,
        segment: u32,
        parent: Option<u32>,
        child: Option<u32>,
        sibling: Option<u32>,
    ) -> bool {
        if matches!(self.state, State::Released) {
            self.generation = generation;
            self.store = store;
            self.segment = segment;
            self.state = State::Initialized;
            self.parent = parent.unwrap_or(u32::MAX);
            self.child = child.unwrap_or(u32::MAX);
            self.sibling = sibling.unwrap_or(u32::MAX);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn update(&mut self, index: u32, segment: u32) -> bool {
        if matches!(self.state, State::Initialized) {
            self.store = index;
            self.segment = segment;
            true
        } else {
            false
        }
    }

    #[inline]
    pub const fn valid(&self, generation: u32) -> bool {
        self.generation == generation && matches!(self.state, State::Initialized)
    }
}

impl Entities {
    pub fn new(capacity: usize) -> Self {
        let mut free = Vec::with_capacity(capacity);
        let mut data = Vec::with_capacity(capacity);
        free.push(Entity::ZERO);
        data.push(Datum::default());
        Self {
            free: (free, 1.into()),
            data: (data, 1.into()),
        }
    }

    pub fn root(&self, entity: Entity) -> Entity {
        match self.parent(entity) {
            Some(parent) => self.root(parent),
            None => entity,
        }
    }

    // TODO: implement other family methods
    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        let datum = self.get_datum(entity)?;
        let parent = self
            .data
            .0
            .get(datum.parent as usize)
            .filter(|datum| datum.valid(entity.generation))?;
        Some(Entity {
            index: datum.parent,
            generation: parent.generation,
        })
    }

    pub fn reserve(&self, entities: &mut [Entity]) -> usize {
        if entities.len() == 0 {
            return 0;
        }

        let count = entities.len() as isize;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = max(min(count, last), 0) as usize;
        for i in 0..count {
            let index = last as usize - i - 1;
            let mut entity = self.free.0[index];
            entity.generation = self.data.0[entity.index as usize].generation + 1;
            entities[i] = entity;
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return count;
        }

        let index = self.data.1.fetch_add(remaining, Ordering::Relaxed);
        for i in 0..remaining {
            entities[count + i] = Entity {
                index: (index + i) as u32,
                generation: 0,
            };
        }
        count
    }

    pub fn resolve(&mut self) {
        let datum = Datum {
            store: 0,
            segment: 0,
            generation: 0,
            parent: u32::MAX,
            child: u32::MAX,
            sibling: u32::MAX,
            state: State::Initialized,
        };
        self.data.0.resize(*self.data.1.get_mut(), datum);

        let free = self.free.1.get_mut();
        let count = max(*free, 0);
        self.free.0.truncate(count as usize);
        *free = self.free.0.len() as isize;
    }

    pub fn release(&mut self, entities: &[Entity]) {
        for entity in entities {
            self.data.0[entity.index as usize].release();
        }
        self.free.0.extend_from_slice(entities);
        *self.free.1.get_mut() = self.free.0.len() as isize;
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data
            .0
            .get(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data
            .0
            .get_mut(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }

    #[inline]
    pub fn get_datum_unchecked(&self, entity: Entity) -> &Datum {
        &self.data.0[entity.index as usize]
    }

    #[inline]
    pub fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        &mut self.data.0[entity.index as usize]
    }
}

impl Default for Entities {
    #[inline]
    fn default() -> Self {
        Self::new(32)
    }
}
