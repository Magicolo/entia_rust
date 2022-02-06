use crate::{self as entia, entity::Entity, Resource};
use std::{
    cmp::{max, min},
    mem::replace,
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};

#[derive(Resource)]
pub struct Entities {
    free: (Vec<Entity>, AtomicIsize),
    data: (Vec<Datum>, AtomicUsize),
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

    pub fn initialize(
        &mut self,
        generation: u32,
        store_index: u32,
        segment_index: u32,
        parent: Option<u32>,
        first_child: Option<u32>,
        last_child: Option<u32>,
        previous_sibling: Option<u32>,
        next_sibling: Option<u32>,
    ) -> bool {
        if self.released() {
            *self = Datum {
                generation,
                store_index,
                segment_index,
                parent: parent.unwrap_or(u32::MAX),
                first_child: first_child.unwrap_or(u32::MAX),
                last_child: last_child.unwrap_or(u32::MAX),
                previous_sibling: previous_sibling.unwrap_or(u32::MAX),
                next_sibling: next_sibling.unwrap_or(u32::MAX),
            };
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, datum: &Self) -> bool {
        if self.initialized() && datum.initialized() {
            self.store_index = datum.store_index;
            self.segment_index = datum.segment_index;
            true
        } else {
            false
        }
    }

    pub fn release(&mut self) -> bool {
        if self.initialized() {
            self.store_index = u32::MAX;
            self.segment_index = u32::MAX;
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
    pub const fn entity(&self, index: u32) -> Entity {
        Entity::new(index, self.generation)
    }

    fn reject(&mut self) -> (u32, u32, u32) {
        (
            replace(&mut self.parent, u32::MAX),
            replace(&mut self.previous_sibling, u32::MAX),
            replace(&mut self.next_sibling, u32::MAX),
        )
    }
}

impl Entities {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            free: (Vec::with_capacity(capacity), 0.into()),
            data: (Vec::with_capacity(capacity), 0.into()),
        }
    }

    pub(crate) fn reserve(&self, entities: &mut [Entity]) -> usize {
        if entities.len() == 0 {
            return 0;
        }

        let count = entities.len() as isize;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = max(min(count, last), 0) as usize;
        for i in 0..count {
            let index = last as usize - i - 1;
            let entity = self.free.0[index];
            // TODO: What to do if there is an overflow?
            // Overflow could be ignored since it is highly unlikely that entities of early generations are still stored somewhere,
            // but this fact could be exploited...
            let datum = &self.data.0[entity.index() as usize];
            entities[i] = Entity::new(entity.index(), datum.generation + 1);
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return count;
        }

        // TODO: What to do if 'index + remaining >= u32::MAX'?
        // Note that 'u32::MAX' is used as a sentinel so it must be an invalid entity index.
        let index = self.data.1.fetch_add(remaining, Ordering::Relaxed);
        for i in 0..remaining {
            entities[count + i] = Entity::new((index + i) as u32, 0);
        }
        count
    }

    pub(crate) fn resolve(&mut self) {
        self.data.0.resize(*self.data.1.get_mut(), Datum::DEFAULT);
        let free = self.free.1.get_mut();
        let count = max(*free, 0) as usize;
        self.free.0.truncate(count);
        *free = self.free.0.len() as isize;
    }

    pub(crate) fn release(&mut self, entities: impl IntoIterator<Item = Entity>) {
        let index = self.free.0.len();
        self.free.0.extend(entities);
        for &entity in &self.free.0[index..] {
            self.data.0[entity.index() as usize].release();
        }
        *self.free.1.get_mut() = self.free.0.len() as isize;
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
                if datum.initialized() && datum.parent == u32::MAX {
                    Some(datum.entity(index as u32))
                } else {
                    None
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
        struct Children<'a>(u32, u32, bool, &'a Entities);

        impl Iterator for Children<'_> {
            type Item = Entity;

            fn next(&mut self) -> Option<Self::Item> {
                if self.2 {
                    None
                } else {
                    let datum = self.3.get_datum_at(self.0)?;
                    let entity = datum.entity(self.0);
                    self.2 = self.0 == self.1;
                    self.0 = datum.next_sibling;
                    Some(entity)
                }
            }
        }

        impl DoubleEndedIterator for Children<'_> {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.2 {
                    None
                } else {
                    let datum = self.3.get_datum_at(self.1)?;
                    let entity = datum.entity(self.1);
                    self.2 = self.0 == self.1;
                    self.1 = datum.previous_sibling;
                    Some(entity)
                }
            }
        }

        let index = self
            .get_datum(entity)
            .map_or((u32::MAX, u32::MAX), |datum| {
                (datum.first_child, datum.last_child)
            });
        Children(index.0, index.1, false, self)
    }

    pub fn siblings(&self, entity: Entity) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.parent(entity)
            .map(|parent| self.children(parent))
            .into_iter()
            .flatten()
            .filter(move |&child| child != entity)
    }

    pub fn ancestors(&self, entity: Entity) -> impl DoubleEndedIterator<Item = Entity> {
        let mut entities = Vec::new();
        self.ascend(
            entity,
            |parent| -> Option<()> {
                entities.push(parent);
                None
            },
            |_| None,
        );
        entities.into_iter()
    }

    pub fn descendants(&self, entity: Entity) -> impl DoubleEndedIterator<Item = Entity> {
        let mut entities = Vec::new();
        self.descend(
            entity,
            |child| -> Option<()> {
                entities.push(child);
                None
            },
            |_| None,
        );
        entities.into_iter()
    }

    pub fn ascend<T>(
        &self,
        entity: Entity,
        mut up: impl FnMut(Entity) -> Option<T>,
        mut down: impl FnMut(Entity) -> Option<T>,
    ) -> Option<T> {
        fn next<T>(
            entities: &Entities,
            entity: Entity,
            up: &mut impl FnMut(Entity) -> Option<T>,
            down: &mut impl FnMut(Entity) -> Option<T>,
        ) -> Option<T> {
            if let Some(parent) = entities.parent(entity) {
                if let Some(value) = up(parent) {
                    return Some(value);
                }
                if let Some(value) = next(entities, parent, up, down) {
                    return Some(value);
                }
                if let Some(value) = down(parent) {
                    return Some(value);
                }
            }
            None
        }

        if self.has(entity) {
            next(self, entity, &mut up, &mut down)
        } else {
            None
        }
    }

    pub fn descend<T>(
        &self,
        entity: Entity,
        mut down: impl FnMut(Entity) -> Option<T>,
        mut up: impl FnMut(Entity) -> Option<T>,
    ) -> Option<T> {
        fn next<T>(
            entities: &Entities,
            entity: Entity,
            down: &mut impl FnMut(Entity) -> Option<T>,
            up: &mut impl FnMut(Entity) -> Option<T>,
        ) -> Option<T> {
            for child in entities.children(entity) {
                if let Some(value) = down(child) {
                    return Some(value);
                }
                if let Some(value) = next(entities, child, down, up) {
                    return Some(value);
                }
                if let Some(value) = up(child) {
                    return Some(value);
                }
            }
            None
        }

        if self.has(entity) {
            next(self, entity, &mut down, &mut up)
        } else {
            None
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
        if let Some(_) = self.ascend(
            parent,
            |parent| if parent == child { Some(()) } else { None },
            |_| None,
        ) {
            return None;
        }

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

impl Default for Entities {
    #[inline]
    fn default() -> Self {
        Self::new(32)
    }
}
