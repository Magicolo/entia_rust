use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::cmp::min;
use std::mem::replace;
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
        for store in self.stores.iter() {
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
        if index + count > self.capacity {
            (index, self.capacity - min(index, self.capacity))
        } else {
            (index, count)
        }
    }

    pub fn resolve(&mut self) {
        let reserved = self.reserved.get_mut();
        let count = self.count + *reserved;

        if self.capacity < count {
            let capacity = next_power_of_2(count as u32 - 1) as usize;
            for store in self.stores.iter() {
                unsafe { store.resize(self.capacity, capacity) };
            }
            self.capacity = capacity;
        }

        self.count += replace(reserved, 0);
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        for store in self.stores.iter() {
            unsafe { (store.0.free)(store.data(), self.count, self.capacity) };
        }
    }
}

impl Store {
    #[inline]
    pub fn new(meta: Meta, capacity: usize) -> Self {
        let pointer = (meta.allocate)(capacity);
        Self(meta, pointer.into())
    }

    #[inline]
    pub const fn meta(&self) -> &Meta {
        &self.0
    }

    #[inline]
    pub fn data(&self) -> *mut () {
        unsafe { *self.1.get() }
    }

    #[inline]
    pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
        debug_assert_eq!(source.0.meta().identifier, target.0.meta().identifier);
        (source.0.meta().copy)(
            (source.0.data(), source.1),
            (target.0.data(), target.1),
            count,
        );
    }

    /// SAFETY: The 'index' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get<T: 'static>(&self, index: usize) -> &mut T {
        debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier);
        &mut *self.data().cast::<T>().add(index)
    }

    /// SAFETY: Both 'index' and 'count' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get_all<T: 'static>(&self, index: usize, count: usize) -> &mut [T] {
        debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier);
        from_raw_parts_mut(self.data().cast::<T>().add(index), count)
    }

    #[inline]
    pub unsafe fn set<T: 'static>(&self, index: usize, item: T) {
        debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier);
        self.data().cast::<T>().add(index).write(item);
    }

    #[inline]
    pub unsafe fn set_any(&self, index: usize, item: Box<dyn Any>) {
        debug_assert_eq!(item.type_id(), self.meta().identifier);
        (self.meta().set)(self.data(), item, index);
    }

    #[inline]
    pub unsafe fn set_all<T: 'static>(&self, index: usize, items: &[T])
    where
        T: Copy,
    {
        debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier);
        let source = items.as_ptr().cast::<T>();
        let target = self.data().cast::<T>().add(index);
        source.copy_to_nonoverlapping(target, items.len());
    }

    /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
    #[inline]
    pub unsafe fn squash(&self, source_index: usize, target_index: usize) {
        let meta = self.meta();
        let data = self.data();
        (meta.drop)(data, target_index, 1);
        (meta.copy)((data, source_index), (data, target_index), 1);
    }

    #[inline]
    pub unsafe fn resize(&self, old_capacity: usize, new_capacity: usize) {
        let meta = self.meta();
        let old_data = self.data();
        let new_data = (self.meta().allocate)(new_capacity);
        (meta.copy)((old_data, 0), (new_data, 0), old_capacity);
        (meta.free)(old_data, 0, old_capacity);
        *self.1.get() = new_data;
    }

    #[inline]
    pub unsafe fn drop(&self, index: usize, count: usize) {
        (self.meta().drop)(self.data(), index, count);
    }
}
