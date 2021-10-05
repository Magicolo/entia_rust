use std::{
    cmp::{max, min},
    iter::from_fn,
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};

use crate::{entity::Entity, resource::Resource};

pub struct Entities {
    pub(crate) free: (Vec<Entity>, AtomicIsize),
    pub(crate) data: (Vec<Datum>, AtomicUsize),
}

#[derive(Clone)]
pub struct Datum {
    pub(crate) store_index: u32,
    pub(crate) segment_index: u32,
    pub(crate) generation: u32,
    pub(crate) parent: u32,
    pub(crate) first_child: u32,
    pub(crate) previous_sibling: u32,
    pub(crate) next_sibling: u32,
    pub(crate) state: State,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum State {
    Released = 0,
    Initialized = 1,
}

pub enum Direction {
    TopDown,
    BottomUp,
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
        self.store_index
    }

    #[inline]
    pub const fn segment(&self) -> u32 {
        self.segment_index
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
        store_index: u32,
        segment_index: u32,
        parent: Option<u32>,
        first_child: Option<u32>,
        previous_sibling: Option<u32>,
        next_sibling: Option<u32>,
    ) -> bool {
        if matches!(self.state, State::Released) {
            self.generation = generation;
            self.store_index = store_index;
            self.segment_index = segment_index;
            self.parent = parent.unwrap_or(u32::MAX);
            self.first_child = first_child.unwrap_or(u32::MAX);
            self.previous_sibling = previous_sibling.unwrap_or(u32::MAX);
            self.next_sibling = next_sibling.unwrap_or(u32::MAX);
            self.state = State::Initialized;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn update(&mut self, index: u32, segment: u32) -> bool {
        if matches!(self.state, State::Initialized) {
            self.store_index = index;
            self.segment_index = segment;
            true
        } else {
            false
        }
    }

    #[inline]
    pub const fn valid(&self, generation: u32) -> bool {
        self.generation == generation && self.initialized()
    }

    #[inline]
    pub const fn initialized(&self) -> bool {
        matches!(self.state, State::Initialized)
    }
}

impl Entities {
    pub fn new(capacity: usize) -> Self {
        Self {
            free: (Vec::with_capacity(capacity), 0.into()),
            data: (Vec::with_capacity(capacity), 0.into()),
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        self.get_datum(entity).is_some()
    }

    pub fn root(&self, entity: Entity) -> Entity {
        self.ancestors(entity).last().unwrap_or(entity)
    }

    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.ancestors(entity).next()
    }

    pub fn ancestors(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.link(entity, |datum| datum.parent, |datum| datum.parent)
    }

    pub fn children(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.link(
            entity,
            |datum| datum.first_child,
            |datum| datum.next_sibling,
        )
    }

    pub fn descend(&self, entity: Entity, direction: Direction, mut each: impl FnMut(Entity)) {
        match direction {
            Direction::TopDown => self.descend_top_down(entity, &mut each),
            Direction::BottomUp => self.descend_bottom_up(entity, &mut each),
        }
    }

    pub fn descendants(
        &self,
        entity: Entity,
        direction: Direction,
    ) -> impl Iterator<Item = Entity> + '_ {
        let mut entities = Vec::new();
        self.descend(entity, direction, |child| entities.push(child));
        entities.into_iter()
    }

    pub fn left_siblings(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.link(
            entity,
            |datum| datum.previous_sibling,
            |datum| datum.previous_sibling,
        )
    }

    pub fn right_siblings(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.link(
            entity,
            |datum| datum.next_sibling,
            |datum| datum.next_sibling,
        )
    }

    pub fn siblings(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.parent(entity)
            .map(|parent| self.children(parent).filter(move |&child| child != entity))
            .into_iter()
            .flatten()
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
            // TODO: What to do if there is an overflow?
            // Overflow could be ignored since it is highly unlikely that entities of early generations are still stored somewhere,
            // but this fact could be exploited...
            // Also, at 'index == 0', a generation of 0 must not be allowed.
            entity.generation = self.data.0[entity.index as usize].generation + 1;
            entities[i] = entity;
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return count;
        }

        // TODO: What to do if 'index + remaining >= u32::MAX'?
        // Note that 'u32::MAX' is used as a sentinel so it must be an invalid entity index.
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
            store_index: 0,
            segment_index: 0,
            generation: 0,
            parent: u32::MAX,
            first_child: u32::MAX,
            previous_sibling: u32::MAX,
            next_sibling: u32::MAX,
            state: State::Initialized,
        };
        self.data.0.resize(*self.data.1.get_mut(), datum);

        let free = self.free.1.get_mut();
        let count = max(*free, 0);
        self.free.0.truncate(count as usize);
        *free = self.free.0.len() as isize;
    }

    pub fn release(&mut self, entities: impl IntoIterator<Item = Entity>) {
        let index = self.free.0.len();
        self.free.0.extend(entities);
        for &entity in &self.free.0[index..] {
            self.data.0[entity.index as usize].release();
        }
        *self.free.1.get_mut() = self.free.0.len() as isize;
    }

    pub fn get_entity_at(&self, index: usize) -> Option<Entity> {
        let datum = self.get_datum_at(index)?;
        Some(Entity {
            index: index as u32,
            generation: datum.generation,
        })
    }

    #[inline]
    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data
            .0
            .get(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }

    #[inline]
    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data
            .0
            .get_mut(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }

    #[inline]
    pub fn get_datum_at(&self, index: usize) -> Option<&Datum> {
        self.data.0.get(index).filter(|datum| datum.initialized())
    }

    #[inline]
    pub fn get_datum_at_mut(&mut self, index: usize) -> Option<&mut Datum> {
        self.data
            .0
            .get_mut(index)
            .filter(|datum| datum.initialized())
    }

    #[inline]
    pub fn get_datum_unchecked(&self, entity: Entity) -> &Datum {
        &self.data.0[entity.index as usize]
    }

    #[inline]
    pub fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        &mut self.data.0[entity.index as usize]
    }

    #[inline]
    fn descend_top_down(&self, parent: Entity, each: &mut impl FnMut(Entity)) {
        for child in self.children(parent) {
            each(child.clone());
            self.descend_top_down(child, each);
        }
    }

    #[inline]
    fn descend_bottom_up(&self, parent: Entity, each: &mut impl FnMut(Entity)) {
        for child in self.children(parent) {
            self.descend_bottom_up(child, each);
            each(child);
        }
    }

    #[inline]
    fn link(
        &self,
        entity: Entity,
        first: fn(&Datum) -> u32,
        next: fn(&Datum) -> u32,
    ) -> impl Iterator<Item = Entity> + '_ {
        let mut index = self.get_datum(entity).map(first).unwrap_or(u32::MAX);
        from_fn(move || {
            let datum = self.data.0.get(index as usize)?;
            let entity = Entity {
                index,
                generation: datum.generation,
            };
            index = next(datum);
            Some(entity)
        })
    }
}

impl Default for Entities {
    #[inline]
    fn default() -> Self {
        Self::new(32)
    }
}
