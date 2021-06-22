use std::cell::UnsafeCell;
use std::cmp::max;
use std::cmp::min;
use std::slice::from_raw_parts_mut;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::usize;

use entia_core::bits::Bits;
use entia_core::utility::next_power_of_2;

use crate::world::Meta;

pub struct Store(Meta, UnsafeCell<*mut ()>);

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) reserved: AtomicUsize,
    pub(crate) types: Bits,
    pub(crate) stores: Box<[Arc<Store>]>,
    capacity: usize,
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
            reserved: 0.into(),
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
            if index == self.count {
                for store in self.stores.iter_mut() {
                    unsafe { store.as_ref().drop(index, 1) };
                }
                false
            } else {
                for store in self.stores.iter_mut() {
                    unsafe { store.squash(self.count, index) };
                }
                true
            }
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

    pub fn reserve(&self, count: usize) -> (usize, usize) {
        let index = self.count + self.reserved.fetch_add(count, Ordering::Relaxed);
        (index, max(self.capacity - min(self.capacity, index), count))
    }

    pub fn resolve(&mut self) {
        let reserved = self.reserved.get_mut();
        let count = self.count + *reserved;

        if self.capacity < count {
            let capacity = next_power_of_2(count as u32) as usize;
            for store in self.stores.iter() {
                unsafe { store.resize(self.count, self.capacity, capacity) };
            }
            self.capacity = capacity;
        }

        self.count += *reserved;
        *reserved = 0;
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
