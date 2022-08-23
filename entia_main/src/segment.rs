use crate::{
    entity::Entity,
    error::{Error, Result},
    identify,
    meta::Meta,
    store::Store,
};
use entia_core::{utility::next_power_of_2, Flags, FullIterator, IntoFlags};
use std::{
    any::TypeId,
    collections::HashSet,
    iter::once,
    mem::replace,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[derive(Clone, Copy)]
pub enum Flag {
    None = 0,
    Clone = 1 << 0,
}

#[derive(Default)]
pub(crate) struct Segments {
    // SAFETY: This vector may only 'push', never 'pop'; otherwise some unsafe index access may become invalid.
    segments: Vec<Segment>,
}

// The 'entity_store' must be kept separate from the 'component_stores' to prevent undesired behavior that may arise
// from using queries such as '&mut Entity' or templates such as 'Add<Entity>'.
pub struct Segment {
    identifier: usize,
    index: usize,
    count: usize,
    entity_store: Arc<Store>,
    component_stores: Box<[Arc<Store>]>,
    component_types: HashSet<TypeId>,
    reserved: AtomicUsize,
    capacity: usize,
}

impl Segments {
    pub fn get_with<I: IntoIterator<Item = TypeId>>(&self, types: I) -> Option<&Segment> {
        Some(&self.segments[self.get_index(&types.into_iter().collect())?])
    }

    pub fn get_or_add<I: IntoIterator<Item = Arc<Meta>>>(
        &mut self,
        entity_meta: Arc<Meta>,
        component_metas: I,
    ) -> &mut Segment {
        let mut component_metas: Vec<_> = component_metas.into_iter().collect();
        let mut component_types = HashSet::new();
        // Ensures there are no duplicates.
        component_metas.retain(|meta| component_types.insert(meta.identifier()));

        let index = match self.get_index(&component_types) {
            Some(index) => index,
            None => {
                let index = self.segments.len();
                let segment = Segment::new(index, 0, entity_meta, component_types, component_metas);
                self.segments.push(segment);
                index
            }
        };
        &mut self.segments[index]
    }

    fn get_index(&self, types: &HashSet<TypeId>) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| segment.component_types() == types)
    }
}

impl Deref for Segments {
    type Target = [Segment];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.segments
    }
}

impl DerefMut for Segments {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.segments
    }
}

impl Segment {
    pub(super) fn new(
        index: usize,
        capacity: usize,
        entity_meta: Arc<Meta>,
        component_types: HashSet<TypeId>,
        mut component_metas: Vec<Arc<Meta>>,
    ) -> Self {
        assert_eq!(entity_meta.identifier(), TypeId::of::<Entity>());
        component_metas.retain(|meta| component_types.contains(&meta.identifier()));

        let entity_store = Arc::new(unsafe { Store::new(entity_meta.clone(), capacity) });
        let component_stores: Box<_> = component_metas
            .into_iter()
            .map(|meta| Arc::new(unsafe { Store::new(meta.clone(), capacity) }))
            .collect();
        Self {
            identifier: identify(),
            index,
            count: 0,
            component_types,
            entity_store,
            component_stores,
            reserved: 0.into(),
            capacity: 0,
        }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub const fn count(&self) -> usize {
        self.count
    }

    #[inline]
    pub const fn component_types(&self) -> &HashSet<TypeId> {
        &self.component_types
    }

    #[inline]
    pub fn component_metas(&self) -> impl FullIterator<Item = Arc<Meta>> + '_ {
        self.component_stores
            .iter()
            .map(|store| Arc::clone(store.meta()))
    }

    pub fn remove_at(&mut self, index: usize) -> bool {
        if index < self.count {
            self.count -= 1;
            if index == self.count {
                for store in self.stores() {
                    unsafe { Store::drop(&store, index, 1) };
                }
                false
            } else {
                for store in self.stores() {
                    unsafe { store.squash(self.count, index, 1) };
                }
                true
            }
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        for store in self.stores() {
            unsafe { Store::drop(&store, 0, self.count) };
        }
        self.count = 0;
    }

    pub fn entity_store(&self) -> Arc<Store> {
        self.entity_store.clone()
    }

    pub fn component_stores(&self) -> impl FullIterator<Item = &Store> {
        self.component_stores.iter().map(AsRef::as_ref)
    }

    pub fn component_store(&self, identifier: TypeId) -> Result<Arc<Store>> {
        self.component_stores
            .iter()
            .find(|store| store.meta().identifier() == identifier)
            .cloned()
            .ok_or(Error::MissingStore {
                identifier,
                segment: self.index,
            })
    }

    pub fn stores(&self) -> impl Iterator<Item = &Store> {
        once(self.entity_store.as_ref()).chain(self.component_stores())
    }

    pub fn reserve(&self, count: usize) -> (usize, usize) {
        let index = self.count + self.reserved.fetch_add(count, Ordering::Relaxed);
        if index + count > self.capacity {
            (index, self.capacity - index.min(self.capacity))
        } else {
            (index, count)
        }
    }

    pub fn resolve(&mut self) {
        self.count += replace(self.reserved.get_mut(), 0);

        if self.capacity < self.count {
            let capacity = next_power_of_2(self.count as u32 - 1) as usize;
            for store in self.stores() {
                unsafe { store.resize(self.capacity, capacity) };
            }
            self.capacity = capacity;
        }
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        for store in self.stores() {
            unsafe { store.free(self.count, self.capacity) };
        }
    }
}

impl IntoFlags for Flag {
    type Value = usize;

    fn flags(self) -> Flags<Self, Self::Value> {
        Flags::new(self as usize)
    }
}
