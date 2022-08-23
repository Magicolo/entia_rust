use entia_core::FullIterator;

use crate::entity::Entity;
use std::{
    mem::replace,
    sync::atomic::{AtomicI64, AtomicU32, Ordering},
};

pub struct Entities {
    free: (Vec<u32>, AtomicI64),
    data: (Vec<Datum>, AtomicU32),
}

impl Default for Entities {
    fn default() -> Self {
        Self::with_capacity(32)
    }
}

#[derive(Clone)]
pub struct Datum {
    // TODO: 'generation' doesn't strictly need to be stored in 'Datum'.
    // An entity could be validated against '*world.segments.get(segment_index)?.stores[0].at::<Entity>(store_index) == entity'.
    // Maybe 'root' or 'state' flags could take the spot of 'generation'?
    pub(crate) generation: u32,
    pub(crate) store_index: u32,
    // TODO: use the last 8 bits of this index to store 'state' information.
    // - This will be used to determine the validity of the datum in a more reliable way.
    // - If the determinant bits are used for state and that a valid datum has the state bits set to 0, the bounds check
    // for segments will also be a validity check. 'world.segments.get(index)' will need to be used everywhere.
    // - What happens when the number of segments overflows u24::MAX?
    pub(crate) segment_index: u32,
    pub(crate) parent: u32,
    pub(crate) first_child: u32,
    pub(crate) last_child: u32,
    pub(crate) previous_sibling: u32,
    pub(crate) next_sibling: u32,
}

impl Datum {
    pub const DEFAULT: Datum = Datum {
        generation: 0,
        store_index: u32::MAX,
        segment_index: u32::MAX,
        parent: u32::MAX,
        first_child: u32::MAX,
        last_child: u32::MAX,
        previous_sibling: u32::MAX,
        next_sibling: u32::MAX,
    };

    #[inline]
    pub fn update(&mut self, store_index: u32, segment_index: u32) -> bool {
        if self.initialized() {
            self.store_index = store_index;
            self.segment_index = segment_index;
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
        self.store_index < u32::MAX && self.segment_index < u32::MAX
    }

    #[inline]
    pub const fn released(&self) -> bool {
        self.store_index == u32::MAX && self.segment_index == u32::MAX
    }

    #[inline]
    pub(crate) const fn entity(&self, index: u32) -> Entity {
        Entity::new(index, self.generation)
    }

    #[inline]
    fn reject(&mut self) -> (u32, u32, u32) {
        (
            replace(&mut self.parent, u32::MAX),
            replace(&mut self.previous_sibling, u32::MAX),
            replace(&mut self.next_sibling, u32::MAX),
        )
    }
}

impl Entities {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            free: (Vec::with_capacity(capacity), 0.into()),
            data: (Vec::with_capacity(capacity), 0.into()),
        }
    }

    pub(crate) fn reserve(&self, entities: &mut [Entity]) -> usize {
        if entities.len() == 0 {
            return 0;
        }

        let count = entities.len() as i64;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = count.min(last).max(0) as usize;
        for i in 0..count {
            let index = last as usize - i - 1;
            let free = self.free.0[index];
            // TODO: What to do if there is an overflow?
            // Overflow could be ignored since it is highly unlikely that entities of early generations are still stored somewhere,
            // but this fact could be exploited...
            let datum = &self.data.0[free as usize];
            entities[i] = Entity::new(free, datum.generation + 1);
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return count;
        }

        // TODO: What to do if 'index + remaining >= u32::MAX'?
        // Note that 'u32::MAX' is used as a sentinel so it must be an invalid entity index.
        let index = self.data.1.fetch_add(remaining as u32, Ordering::Relaxed);
        for i in 0..remaining {
            entities[count + i] = Entity::new(index + i as u32, 0);
        }
        count
    }

    pub(crate) fn resolve(&mut self) {
        let reserved = *self.data.1.get_mut() as usize;
        self.data.0.resize(reserved, Datum::DEFAULT);
        let free = self.free.1.get_mut();
        let count = (*free).max(0) as usize;
        self.free.0.truncate(count);
        *free = self.free.0.len() as i64;
    }

    pub(crate) fn release(&mut self, entities: impl IntoIterator<Item = Entity>) {
        let index = self.free.0.len();
        let indices = entities.into_iter().map(|entity| entity.index());
        self.free.0.extend(indices);
        for &free in &self.free.0[index..] {
            self.data.0[free as usize].update(u32::MAX, u32::MAX);
        }
        *self.free.1.get_mut() = self.free.0.len() as i64;
    }

    #[inline]
    pub(crate) fn initialize(&mut self, index: u32, datum: Datum) -> Option<Datum> {
        let target = self.data.0.get_mut(index as usize)?;
        if target.released() {
            Some(replace(target, datum))
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.get_datum_at(entity.index())
            .filter(|datum| datum.valid(entity.generation()))
    }

    #[inline]
    pub(crate) fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.get_datum_at_mut(entity.index())
            .filter(|datum| datum.valid(entity.generation()))
    }

    #[inline]
    pub(crate) fn get_datum_at(&self, index: u32) -> Option<&Datum> {
        self.data.0.get(index as usize)
    }

    #[inline]
    pub(crate) fn get_datum_at_mut(&mut self, index: u32) -> Option<&mut Datum> {
        self.data.0.get_mut(index as usize)
    }

    #[inline]
    pub fn has(&self, entity: Entity) -> bool {
        self.get_datum(entity).is_some()
    }

    pub fn roots(&self) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.data
            .0
            .iter()
            .enumerate()
            .filter_map(move |(index, datum)| {
                let entity = datum.entity(index as u32);
                match self.parent(entity) {
                    Some(_) => None,
                    None => Some(entity),
                }
            })
    }

    pub fn root(&self, mut entity: Entity) -> Entity {
        // Only the entry entity needs to be validated; linked entities can be assumed to be valid.
        if let Some(datum) = self.get_datum(entity) {
            let mut index = datum.parent;
            while let Some(datum) = self.get_datum_at(index) {
                entity = datum.entity(index);
                index = datum.parent;
            }
        }
        entity
    }

    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        let datum = self.get_datum(entity)?;
        let parent = self.get_datum_at(datum.parent)?;
        Some(parent.entity(datum.parent))
    }

    pub fn children(&self, entity: Entity) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        struct Children<'a>(u32, u32, &'a Entities);

        impl Iterator for Children<'_> {
            type Item = Entity;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                let datum = self.2.get_datum_at(self.0)?;
                let entity = datum.entity(self.0);
                self.0 = if self.0 == self.1 {
                    u32::MAX
                } else {
                    datum.next_sibling
                };
                Some(entity)
            }
        }

        impl DoubleEndedIterator for Children<'_> {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                let datum = self.2.get_datum_at(self.1)?;
                let entity = datum.entity(self.1);
                self.1 = if self.0 == self.1 {
                    u32::MAX
                } else {
                    datum.previous_sibling
                };
                Some(entity)
            }
        }

        let index = self
            .get_datum(entity)
            .map_or((u32::MAX, u32::MAX), |datum| {
                (datum.first_child, datum.last_child)
            });
        Children(index.0, index.1, self)
    }

    pub fn siblings(&self, entity: Entity) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.parent(entity)
            .map(|parent| self.children(parent))
            .into_iter()
            .flatten()
            .filter(move |&child| child != entity)
    }

    pub fn ancestors(&self, entity: Entity) -> impl FullIterator<Item = Entity> {
        let mut entities = Vec::new();
        self.ascend(entity, |parent| entities.push(parent), |_| {});
        entities.into_iter()
    }

    pub fn descendants(&self, entity: Entity) -> impl FullIterator<Item = Entity> {
        let mut entities = Vec::new();
        self.descend(entity, |child| entities.push(child), |_| {});
        entities.into_iter()
    }

    pub fn ascend<U: FnMut(Entity), D: FnMut(Entity)>(
        &self,
        entity: Entity,
        mut up: U,
        mut down: D,
    ) {
        self.try_ascend(
            entity,
            (),
            |entity, _| Ok::<(), ()>(up(entity)),
            |entity, _| Ok::<(), ()>(down(entity)),
        )
        .unwrap_or(())
    }

    pub fn try_ascend<
        T,
        E,
        U: FnMut(Entity, T) -> Result<T, E>,
        D: FnMut(Entity, T) -> Result<T, E>,
    >(
        &self,
        entity: Entity,
        state: T,
        mut up: U,
        mut down: D,
    ) -> Result<T, E> {
        fn next<T, E>(
            entities: &Entities,
            entity: Entity,
            mut state: T,
            up: &mut impl FnMut(Entity, T) -> Result<T, E>,
            down: &mut impl FnMut(Entity, T) -> Result<T, E>,
        ) -> Result<T, E> {
            if let Some(parent) = entities.parent(entity) {
                state = up(parent, state)?;
                state = next(entities, parent, state, up, down)?;
                state = down(parent, state)?;
            }
            Ok(state)
        }

        next(self, entity, state, &mut up, &mut down)
    }

    pub fn descend<D: FnMut(Entity), U: FnMut(Entity)>(
        &self,
        entity: Entity,
        mut down: D,
        mut up: U,
    ) {
        self.try_descend(
            entity,
            (),
            |entity, _| Ok::<(), ()>(down(entity)),
            |entity, _| Ok::<(), ()>(up(entity)),
        )
        .unwrap_or(())
    }

    pub fn try_descend<
        S,
        E,
        D: FnMut(Entity, S) -> Result<S, E>,
        U: FnMut(Entity, S) -> Result<S, E>,
    >(
        &self,
        entity: Entity,
        state: S,
        mut down: D,
        mut up: U,
    ) -> Result<S, E> {
        fn next<S, E>(
            entities: &Entities,
            entity: Entity,
            mut state: S,
            down: &mut impl FnMut(Entity, S) -> Result<S, E>,
            up: &mut impl FnMut(Entity, S) -> Result<S, E>,
        ) -> Result<S, E> {
            for child in entities.children(entity) {
                state = down(child, state)?;
                state = next(entities, child, state, down, up)?;
                state = up(child, state)?;
            }
            Ok(state)
        }

        if self.has(entity) {
            next(self, entity, state, &mut down, &mut up)
        } else {
            Ok(state)
        }
    }

    pub fn adopt_at(&mut self, parent: Entity, child: Entity, index: usize) -> Option<()> {
        if index == 0 {
            self.adopt_first(parent, child)
        } else if index > u32::MAX as usize {
            self.adopt_last(parent, child)
        } else {
            let sibling = self.children(parent).nth(index);
            if let Some(sibling) = sibling {
                self.adopt_before(sibling, child)
            } else {
                self.adopt_last(parent, child)
            }
        }
    }

    pub fn adopt_first(&mut self, parent: Entity, child: Entity) -> Option<()> {
        self.detach_checked(parent, child)?;

        let parent_datum = self.get_datum_at_mut(parent.index()).unwrap();
        let first_child = parent_datum.first_child;
        parent_datum.first_child = child.index();
        if parent_datum.last_child == u32::MAX {
            // Happens when the parent has no children.
            parent_datum.last_child = child.index();
        }

        if let Some(first) = self.get_datum_at_mut(first_child) {
            first.previous_sibling = child.index();
        }

        let child_datum = self.get_datum_at_mut(child.index()).unwrap();
        child_datum.parent = parent.index();
        child_datum.previous_sibling = u32::MAX;
        child_datum.next_sibling = first_child;
        Some(())
    }

    pub fn adopt_last(&mut self, parent: Entity, child: Entity) -> Option<()> {
        self.detach_checked(parent, child)?;

        let parent_datum = self.get_datum_at_mut(parent.index()).unwrap();
        let last_child = parent_datum.last_child;
        parent_datum.last_child = child.index();
        if parent_datum.first_child == u32::MAX {
            // Happens when the parent has no children.
            parent_datum.first_child = child.index();
        }

        if let Some(last) = self.get_datum_at_mut(last_child) {
            last.next_sibling = child.index();
        }

        let child_datum = self.get_datum_at_mut(child.index())?;
        child_datum.parent = parent.index();
        child_datum.previous_sibling = last_child;
        child_datum.next_sibling = u32::MAX;
        Some(())
    }

    pub fn adopt_before(&mut self, sibling: Entity, child: Entity) -> Option<()> {
        let parent = self.parent(sibling)?;
        self.detach_checked(parent, child)?;

        let parent_datum = self.get_datum_at_mut(parent.index()).unwrap();
        // No need to check 'last_child == u32::MAX' since this 'parent' must have at least one child (the 'sibling').
        if parent_datum.first_child == sibling.index() {
            parent_datum.first_child = child.index();
        }

        let sibling_datum = self.get_datum_at_mut(sibling.index()).unwrap();
        let previous_sibling = sibling_datum.previous_sibling;
        sibling_datum.previous_sibling = child.index();
        if let Some(previous) = self.get_datum_at_mut(previous_sibling) {
            previous.next_sibling = child.index();
        }

        let child_datum = self.get_datum_at_mut(child.index()).unwrap();
        child_datum.parent = parent.index();
        child_datum.previous_sibling = previous_sibling;
        child_datum.next_sibling = sibling.index();
        Some(())
    }

    pub fn adopt_after(&mut self, sibling: Entity, child: Entity) -> Option<()> {
        let parent = self.parent(sibling)?;
        self.detach_checked(parent, child)?;

        let parent_datum = self.get_datum_at_mut(parent.index()).unwrap();
        // No need to check 'first_child == u32::MAX' since this 'parent' must have at least one child (the 'sibling').
        if parent_datum.last_child == sibling.index() {
            parent_datum.last_child = child.index();
        }

        let sibling_datum = self.get_datum_at_mut(sibling.index()).unwrap();
        let next_sibling = sibling_datum.next_sibling;
        sibling_datum.next_sibling = child.index();
        if let Some(next) = self.get_datum_at_mut(next_sibling) {
            next.previous_sibling = child.index();
        }

        let child_datum = self.get_datum_at_mut(child.index()).unwrap();
        child_datum.parent = parent.index();
        child_datum.previous_sibling = sibling.index();
        child_datum.next_sibling = next_sibling;
        Some(())
    }

    pub fn reject_at(&mut self, parent: Entity, index: usize) -> Option<Entity> {
        let child = self.children(parent).nth(index)?;
        self.reject(child);
        Some(child)
    }

    pub fn reject_first(&mut self, parent: Entity) -> Option<Entity> {
        let child = self.children(parent).next()?;
        self.reject(child);
        Some(child)
    }

    pub fn reject_last(&mut self, parent: Entity) -> Option<Entity> {
        let child = self.children(parent).next_back()?;
        self.reject(child);
        Some(child)
    }

    pub fn reject_all(&mut self, parent: Entity) -> Option<usize> {
        let parent_datum = self.get_datum_mut(parent)?;
        let first_child = parent_datum.first_child;
        parent_datum.first_child = u32::MAX;
        parent_datum.last_child = u32::MAX;

        let mut count = 0;
        let mut index = first_child;
        while let Some(datum) = self.get_datum_at_mut(index) {
            let next = datum.next_sibling;
            datum.parent = u32::MAX;
            datum.previous_sibling = u32::MAX;
            datum.next_sibling = u32::MAX;
            index = next;
            count += 1;
        }
        Some(count)
    }

    pub fn reject(&mut self, child: Entity) -> Option<bool> {
        let datum = self.get_datum_mut(child)?;
        let (parent, previous_sibling, next_sibling) = datum.reject();
        self.detach_unchecked(parent, child.index(), previous_sibling, next_sibling)?;
        Some(true)
    }

    fn detach_checked(&mut self, parent: Entity, child: Entity) -> Option<()> {
        // A parent entity can adopt an entity that is already its child. In that case, that entity will simply be moved.
        if parent.index() == child.index() {
            // An entity cannot adopt itself.
            // If generations don't match, then one of the entities is invalid, thus adoption also fails.
            return None;
        }

        // An entity cannot adopt an ancestor.
        self.try_ascend(
            parent,
            (),
            |parent, _| if parent == child { Err(()) } else { Ok(()) },
            |_, _| Ok(()),
        )
        .ok()?;

        let &Datum {
            parent,
            previous_sibling,
            next_sibling,
            ..
        } = self.get_datum(child)?;
        // The 'reject' step fails when the entity is a root which is fine here.
        self.detach_unchecked(parent, child.index(), previous_sibling, next_sibling);
        Some(())
    }

    fn detach_unchecked(
        &mut self,
        parent: u32,
        child: u32,
        previous_sibling: u32,
        next_sibling: u32,
    ) -> Option<()> {
        let parent = self.get_datum_at_mut(parent)?;
        if parent.first_child == child {
            parent.first_child = next_sibling;
        }
        if parent.last_child == child {
            parent.last_child = previous_sibling;
        }

        if let Some(previous) = self.get_datum_at_mut(previous_sibling) {
            previous.next_sibling = next_sibling;
        }

        if let Some(next) = self.get_datum_at_mut(next_sibling) {
            next.previous_sibling = previous_sibling;
        }

        Some(())
    }
}
