use crate::core::bits::*;
use crate::core::utility::*;
use crate::world::*;
use std::ptr::NonNull;
use std::sync::Arc;
use std::usize;

pub struct Store(Arc<Meta>, NonNull<()>);

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) capacity: usize,
    pub(crate) types: Bits,
    pub(crate) stores: Box<[Store]>,
}

pub struct Move {
    source: usize,
    target: usize,
    to_copy: Vec<(Store, Store)>,
    to_drop: Vec<Store>,
}

unsafe impl Sync for Store {}
unsafe impl Send for Store {}

impl Segment {
    pub(crate) fn new(
        index: usize,
        types: Bits,
        metas: impl IntoIterator<Item = Arc<Meta>>,
        capacity: usize,
    ) -> Self {
        let stores = metas
            .into_iter()
            .map(|meta| Store::new(meta.clone(), capacity))
            .collect();
        Self {
            index,
            count: 0,
            capacity,
            types,
            stores,
        }
    }

    #[inline]
    pub fn has(&self, meta: &Meta) -> bool {
        self.types.has(meta.index)
    }

    pub fn remove_at(&mut self, index: usize) -> bool {
        if index < self.count {
            self.count -= 1;
            let last = self.count;
            for store in self.stores.iter_mut() {
                unsafe { store.squash(last, index) };
            }
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        for store in self.stores.iter_mut() {
            unsafe { store.drop(0, self.count) };
        }
        self.count = 0;
    }

    pub fn store(&self, meta: &Meta) -> Option<&Store> {
        if self.types.has(meta.index) {
            for store in self.stores.iter() {
                if store.0.identifier == meta.identifier && store.0.index == meta.index {
                    return Some(store);
                }
            }
        }
        None
    }

    pub fn reserve(&mut self, count: usize) -> usize {
        let index = self.count;
        self.count += count;
        self.ensure(self.count);
        index
    }

    pub fn ensure(&mut self, capacity: usize) -> bool {
        if self.capacity <= capacity {
            false
        } else {
            self.capacity = next_power_of_2(capacity as u32) as usize;
            for store in self.stores.iter_mut() {
                unsafe { store.resize(self.count, self.capacity) };
            }
            true
        }
    }
}

impl Store {
    #[inline]
    pub fn new(meta: Arc<Meta>, capacity: usize) -> Self {
        let data = (meta.allocate)(capacity);
        Self(meta, data)
    }

    #[inline]
    pub unsafe fn copy(source: (&Store, usize), target: (&Store, usize), count: usize) {
        (source.0 .0.copy)((source.0 .1, source.1), (target.0 .1, target.1), count);
    }

    #[inline]
    pub unsafe fn get<T>(&mut self) -> *mut T {
        self.1.cast().as_ptr()
    }

    #[inline]
    pub unsafe fn at<T>(&self, index: usize) -> &mut T {
        &mut *self.1.cast::<T>().as_ptr().add(index)
    }

    #[inline]
    pub unsafe fn set<T>(&mut self, index: usize, item: T) {
        self.1.cast::<T>().as_ptr().add(index).write(item);
    }

    #[inline]
    pub unsafe fn squash(&mut self, source: usize, target: usize) {
        (self.0.drop)(self.1, target, 1);
        (self.0.copy)((self.1, source), (self.1, target), 1);
    }

    #[inline]
    pub unsafe fn resize(&mut self, count: usize, capacity: usize) {
        let store = (self.0.allocate)(capacity);
        (self.0.copy)((self.1, 0), (store, 0), count);
        self.1 = store;
    }

    #[inline]
    pub unsafe fn drop(&mut self, index: usize, count: usize) {
        (self.0.drop)(self.1, index, count);
    }

    pub unsafe fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}

impl Move {
    pub fn new(source: &Segment, target: &Segment) -> Self {
        if source.index == target.index {
            Move {
                source: source.index,
                target: target.index,
                to_copy: Vec::new(),
                to_drop: Vec::new(),
            }
        } else {
            let mut to_copy = Vec::new();
            let mut to_drop = Vec::new();
            for source in source.stores.iter() {
                if let Some(target) = target.store(&source.0) {
                    to_copy.push(unsafe { (source.clone(), target.clone()) });
                } else {
                    to_drop.push(unsafe { source.clone() })
                }
            }
            Move {
                source: source.index,
                target: target.index,
                to_copy,
                to_drop,
            }
        }
    }

    pub fn apply(&mut self, index: usize, count: usize, world: &mut World) -> Option<usize> {
        let indices = (self.source, self.target);
        if indices.0 == indices.1 {
            Some(index)
        } else if let Some((source, target)) = get_mut2(&mut world.segments, indices) {
            source.count -= count;
            let source_index = source.count;
            let target_index = target.reserve(count);
            for (source_store, target_store) in self.to_copy.iter_mut() {
                unsafe { Store::copy((source_store, index), (target_store, target_index), count) };
                unsafe { Store::copy((source_store, source_index), (source_store, index), count) };
            }

            for store in self.to_drop.iter_mut() {
                unsafe { store.drop(source_index, 1) };
            }
            Some(target_index)
        } else {
            None
        }
    }

    #[inline]
    pub const fn source(&self) -> usize {
        self.source
    }

    #[inline]
    pub const fn target(&self) -> usize {
        self.target
    }
}
