use crate::{
    error::{Error, Result},
    identify,
    meta::Meta,
    store::Store,
};
use entia_core::{utility::next_power_of_2, Flags, IntoFlags};
use std::{
    any::TypeId,
    collections::HashSet,
    iter::once,
    mem::replace,
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

// SAFETY: The inner vector may only 'push', never 'pop'; otherwise some unsafe index access may become invalid.
#[derive(Default)]
pub struct Segments(Vec<Segment>);

// The 'entity_store' must be kept separate from the 'component_stores' to prevent undesired behavior that may arise
// from using queries such as '&mut Entity' or templates such as 'Add<Entity>'.
pub struct Segment {
    identifier: usize,
    index: usize,
    count: usize,
    flags: Flags<Flag>,
    entity_store: Arc<Store>,
    component_stores: Box<[Arc<Store>]>,
    component_types: HashSet<TypeId>,
    reserved: AtomicUsize,
    capacity: usize,
}

impl Segment {
    pub(super) fn new(
        index: usize,
        capacity: usize,
        entity_meta: Arc<Meta>,
        component_types: HashSet<TypeId>,
        component_metas: &[Arc<Meta>],
    ) -> Self {
        let entity_store = Arc::new(unsafe { Store::new(entity_meta.clone(), capacity) });
        // Iterate over all metas in order to have them consistently ordered.
        let component_stores: Box<_> = component_metas
            .iter()
            .filter(|meta| component_types.contains(&meta.identifier()))
            .map(|meta| Arc::new(unsafe { Store::new(meta.clone(), capacity) }))
            .collect();
        let mut flags = Flag::None.flags();
        if component_stores
            .iter()
            .all(|store| store.meta().cloner.is_some())
        {
            flags |= Flag::Clone;
        }

        Self {
            identifier: identify(),
            index,
            count: 0,
            flags,
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
    pub fn can_clone(&self) -> bool {
        self.flags.has_all(Flag::Clone)
    }

    #[inline]
    pub const fn component_types(&self) -> &HashSet<TypeId> {
        &self.component_types
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

    pub fn component_stores(&self) -> impl ExactSizeIterator<Item = &Store> {
        self.component_stores.iter().map(AsRef::as_ref)
    }

    pub fn component_store(&self, meta: &Meta) -> Result<Arc<Store>> {
        let identifier = meta.identifier();
        self.component_stores
            .iter()
            .find(|store| store.meta().identifier() == identifier)
            .cloned()
            .ok_or(Error::MissingStore {
                name: meta.name(),
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
