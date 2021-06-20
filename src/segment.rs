use std::cell::UnsafeCell;
use std::cmp::max;
use std::cmp::min;
use std::slice::from_raw_parts_mut;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::usize;

use entia_core::bits::Bits;
use entia_core::utility::get_mut2;
use entia_core::utility::next_power_of_2;

use crate::world::{Meta, World};

pub struct Store(Meta, UnsafeCell<*mut ()>);

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: AtomicUsize,
    pub(crate) types: Bits,
    pub(crate) stores: Box<[Arc<Store>]>,
    capacity: usize,
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
            count: 0.into(),
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
        let count = self.count.get_mut();
        if index < *count {
            *count -= 1;
            if index == *count {
                for store in self.stores.iter_mut() {
                    unsafe { store.as_ref().drop(index, 1) };
                }
                false
            } else {
                for store in self.stores.iter_mut() {
                    unsafe { store.squash(*count, index) };
                }
                true
            }
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        let count = self.count.get_mut();
        for store in self.stores.iter_mut() {
            unsafe { store.as_ref().drop(0, *count) };
        }
        *count = 0;
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

    pub fn prepare(&self, count: usize) -> (usize, usize) {
        let index = self.count.fetch_add(count, Ordering::Relaxed);
        (index, max(self.capacity - min(self.capacity, index), count))
    }

    pub fn resolve(&mut self) {
        let count = *self.count.get_mut();
        if self.capacity < count {
            self.resize(count, next_power_of_2(self.capacity as u32) as usize);
        }
    }

    pub fn reserve(&mut self, count: usize) -> usize {
        let (index, _) = self.prepare(count);
        self.resolve();
        index
    }

    pub fn ensure(&mut self, capacity: usize) {
        if self.capacity < capacity {
            let count = *self.count.get_mut();
            self.resize(count, next_power_of_2(self.capacity as u32) as usize);
        }
    }

    fn resize(&mut self, count: usize, capacity: usize) {
        for store in self.stores.iter() {
            unsafe { store.resize(count, self.capacity, capacity) };
        }
        self.capacity = capacity;
    }
}

impl Store {
    #[inline]
    pub fn new(meta: Meta, capacity: usize) -> Self {
        let pointer = (meta.allocate)(capacity);
        Self(meta, pointer.into())
    }

    #[inline]
    pub unsafe fn copy(source: (&Store, usize), target: (&Store, usize), count: usize) {
        (source.0 .0.copy)(
            (source.0.pointer(), source.1),
            (target.0.pointer(), target.1),
            count,
        );
    }

    /// SAFETY: The 'index' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get<T>(&self, index: usize) -> &mut T {
        &mut *self.pointer().cast::<T>().add(index)
    }

    /// SAFETY: Both 'index' and 'count' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get_all<T>(&self, index: usize, count: usize) -> &mut [T] {
        from_raw_parts_mut(self.pointer().cast::<T>().add(index), count)
    }

    #[inline]
    pub unsafe fn set<T>(&self, index: usize, item: T) {
        self.pointer().cast::<T>().add(index).write(item);
    }

    #[inline]
    pub unsafe fn set_all<T>(&self, index: usize, items: &[T])
    where
        T: Copy,
    {
        let source = items.as_ptr().cast::<T>();
        let target = self.pointer().cast::<T>().add(index);
        source.copy_to_nonoverlapping(target, items.len());
    }

    /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
    #[inline]
    pub unsafe fn squash(&self, source: usize, target: usize) {
        let data = self.pointer();
        (self.0.drop)(data, target, 1);
        (self.0.copy)((data, source), (data, target), 1);
    }

    #[inline]
    pub unsafe fn resize(&self, count: usize, old: usize, new: usize) {
        let pointer = self.pointer();
        let data = (self.0.allocate)(new);
        (self.0.copy)((pointer, 0), (data, 0), count);
        (self.0.free)(pointer, old);
        *self.1.get() = data;
    }

    #[inline]
    pub unsafe fn drop(&self, index: usize, count: usize) {
        (self.0.drop)(self.pointer(), index, count);
    }

    #[inline]
    unsafe fn pointer(&self) -> *mut () {
        *self.1.get()
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
            let source_count = source.count.get_mut();
            *source_count -= count;
            let source_index = *source_count;
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
