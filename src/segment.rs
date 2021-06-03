use crate::core::bits::*;
use crate::core::utility::*;
use crate::world::*;
use std::cell::UnsafeCell;
use std::ptr::{copy_nonoverlapping, NonNull};
use std::slice::from_raw_parts_mut;
use std::sync::Arc;
use std::usize;

pub struct Store(Meta, UnsafeCell<NonNull<()>>);

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) capacity: usize,
    pub(crate) types: Bits,
    pub(crate) stores: Box<[Arc<Store>]>,
}

pub struct Move {
    source: usize,
    target: usize,
    to_copy: Vec<(Arc<Store>, Arc<Store>)>,
    to_drop: Vec<Arc<Store>>,
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
            .map(|meta| Store::new(meta.as_ref().clone(), capacity).into())
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
            unsafe { store.as_ref().drop(0, self.count) };
        }
        self.count = 0;
    }

    pub fn store(&self, meta: &Meta) -> Option<Arc<Store>> {
        if self.types.has(meta.index) {
            for store in self.stores.iter() {
                if store.0.identifier == meta.identifier && store.0.index == meta.index {
                    return Some(store.clone());
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
            let capacity = next_power_of_2(capacity as u32) as usize;
            for store in self.stores.iter() {
                unsafe { store.resize(self.count, self.capacity, capacity) };
            }
            self.capacity = capacity;
            true
        }
    }
}

impl Store {
    #[inline]
    pub fn new(meta: Meta, capacity: usize) -> Self {
        let data = (meta.allocate)(capacity);
        Self(meta, data.into())
    }

    #[inline]
    pub unsafe fn copy(source: (&Store, usize), target: (&Store, usize), count: usize) {
        (source.0 .0.copy)(
            (*source.0.data(), source.1),
            (*target.0.data(), target.1),
            count,
        );
    }

    /// SAFETY: The 'count' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get<T>(&self, count: usize) -> &mut [T] {
        from_raw_parts_mut(self.data().cast().as_ptr(), count)
    }

    /// SAFETY: The 'items' reference must not point into 'self' (ex: through usage of 'self.get(usize)').
    #[inline]
    pub unsafe fn set<T>(&self, index: usize, items: &[T]) {
        let pointer = self.data().cast::<T>().as_ptr().add(index);
        copy_nonoverlapping(items.as_ptr(), pointer, items.len());
    }

    /// SAFETY: The 'index' must be within the bounds of the store.
    #[inline]
    pub unsafe fn at<T>(&self, index: usize) -> &mut T {
        &mut *self.data().cast::<T>().as_ptr().add(index)
    }

    /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
    #[inline]
    pub unsafe fn squash(&self, source: usize, target: usize) {
        let data = *self.data();
        (self.0.drop)(data, target, 1);
        (self.0.copy)((data, source), (data, target), 1);
    }

    #[inline]
    pub unsafe fn resize(&self, count: usize, old: usize, new: usize) {
        let data = self.data();
        let new = (self.0.allocate)(new);
        (self.0.copy)((*data, 0), (new, 0), count);
        (self.0.free)(*data, old);
        *data = new;
    }

    #[inline]
    pub unsafe fn drop(&self, index: usize, count: usize) {
        (self.0.drop)(*self.data(), index, count);
    }

    #[inline]
    unsafe fn data(&self) -> &mut NonNull<()> {
        &mut *self.1.get()
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
                    to_copy.push((source.clone(), target));
                } else {
                    to_drop.push(source.clone())
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
                unsafe { store.as_ref().drop(source_index, 1) };
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
