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
    index: u32,
    segment: u32,
    generation: u32,
    state: u8,
}

impl Resource for Entities {}

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

    pub fn reserve(&mut self, entities: &mut [Entity]) -> usize {
        if entities.len() == 0 {
            return 0;
        }

        let count = entities.len() as isize;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = max(min(count, last), 0) as usize;
        for i in 0..count {
            let index = last as usize - i - 1;
            let mut entity = self.free.0[index];
            entity.generation = self.data.0[entity.index as usize].reserve();
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

    pub fn release(&mut self, entities: &[Entity]) {
        for entity in entities {
            self.data.0[entity.index as usize].release();
        }
        self.free.0.extend_from_slice(entities);
        *self.free.1.get_mut() = self.free.0.len() as isize;
    }

    pub fn resolve(&mut self) {
        let count = max(*self.free.1.get_mut(), 0);
        self.free.0.truncate(count as usize);
        *self.free.1.get_mut() = self.free.0.len() as isize;

        let count = *self.data.1.get_mut();
        let datum = Datum {
            index: 0,
            segment: 0,
            generation: 0,
            state: 1,
        };
        self.data.0.resize(count, datum);
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
    #[allow(dead_code)]
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
