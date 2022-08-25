use crate::{
    entity::Entity,
    error::{Error, Result},
    identify,
    meta::{Meta, Metas},
    resource::Resource,
    store::Store,
};
use entia_core::{utility::next_power_of_2, Flags, FullIterator, IntoFlags};
use std::{
    any::TypeId,
    collections::HashSet,
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
    Default = 1 << 1,
}

#[derive(Default)]
pub struct Segments {
    // SAFETY: This vector may only 'push', never 'pop'; otherwise some unsafe index access may become invalid.
    segments: Vec<Segment>,
}

impl Resource for Segments {}

// The 'entity_store' must be kept separate from the 'component_stores' to prevent undesired behavior that may arise
// from using queries such as '&mut Entity' or templates such as 'Add<Entity>'.
pub struct Segment {
    identifier: usize,
    index: usize,
    count: usize,
    flags: Flags<Flag>,
    entity_store: Arc<Store>,
    stores: Box<[Arc<Store>]>,
    types: HashSet<TypeId>,
    reserved: AtomicUsize,
    capacity: usize,
}

impl Segments {
    pub fn get_with<I: IntoIterator<Item = TypeId>>(&self, types: I) -> Option<&Segment> {
        Some(&self.segments[self.get_index(&types.into_iter().collect())?])
    }

    pub fn get_or_add<I: IntoIterator<Item = Arc<Meta>>>(
        &mut self,
        component_metas: I,
        metas: &Metas,
    ) -> &mut Segment {
        let mut metas: Vec<_> = [metas.entity()]
            .into_iter()
            .chain(component_metas)
            .collect();
        let mut types = HashSet::new();
        // Ensures there are no duplicates.
        metas.retain(|meta| types.insert(meta.identifier()));

        let index = match self.get_index(&types) {
            Some(index) => index,
            None => {
                let index = self.segments.len();
                let segment = Segment::new(index, 0, types, metas);
                self.segments.push(segment);
                index
            }
        };
        &mut self.segments[index]
    }

    fn get_index(&self, types: &HashSet<TypeId>) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| segment.types() == types)
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
        types: HashSet<TypeId>,
        mut metas: Vec<Arc<Meta>>,
    ) -> Self {
        metas.retain(|meta| types.contains(&meta.identifier()));

        let mut flags = Flags::new(0);
        if metas.iter().all(|meta| meta.cloner.is_some()) {
            flags |= Flag::Clone;
        }
        if metas.iter().all(|meta| meta.defaulter.is_some()) {
            flags |= Flag::Default;
        }

        let component_stores: Box<_> = metas
            .into_iter()
            .map(|meta| Arc::new(unsafe { Store::new(meta.clone(), capacity) }))
            .collect();
        let entity_store = component_stores
            .iter()
            .find(|store| store.meta().is::<Entity>())
            .cloned()
            .expect("Entity store is required.");
        Self {
            identifier: identify(),
            index,
            count: 0,
            flags,
            types,
            entity_store,
            stores: component_stores,
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
    pub const fn types(&self) -> &HashSet<TypeId> {
        &self.types
    }

    pub fn entity_store(&self) -> &Arc<Store> {
        &self.entity_store
    }

    pub fn metas(&self) -> impl FullIterator<Item = Arc<Meta>> + '_ {
        self.stores.iter().map(|store| Arc::clone(store.meta()))
    }

    pub fn stores(&self) -> impl FullIterator<Item = &Store> {
        self.stores.iter().map(AsRef::as_ref)
    }

    pub fn store(&self, identifier: TypeId) -> Result<&Arc<Store>> {
        self.stores
            .iter()
            .find(|store| store.meta().identifier() == identifier)
            .ok_or(Error::MissingStore {
                identifier,
                segment: self.index,
            })
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

        if self.count > self.capacity {
            let capacity = next_power_of_2(self.count as u32 - 1) as usize;
            for store in self.stores() {
                unsafe { store.grow(self.capacity, capacity) };
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
