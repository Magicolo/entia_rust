use std::{
    any::{type_name, Any, TypeId},
    cell::UnsafeCell,
    cmp::min,
    collections::HashMap,
    mem::{needs_drop, replace, size_of, ManuallyDrop},
    ptr::{copy, drop_in_place, slice_from_raw_parts_mut},
    slice::from_raw_parts_mut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use self::segment::*;
use self::store::*;
use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Context, Get, Inject},
};
use entia_core::{bits::Bits, utility::next_power_of_2};

#[derive(Clone)]
pub struct Meta {
    pub(crate) identifier: TypeId,
    pub(crate) name: &'static str,
    pub(crate) index: usize,
    pub(crate) size: usize,
    pub(crate) allocate: fn(usize) -> *mut (),
    pub(crate) free: unsafe fn(*mut (), usize, usize),
    pub(crate) copy: unsafe fn((*const (), usize), (*mut (), usize), usize),
    pub(crate) drop: unsafe fn(*mut (), usize, usize),
    pub(crate) set: unsafe fn(*mut (), Box<dyn Any>, usize),
}

pub struct World {
    identifier: usize,
    version: usize,
    pub(crate) segments: Vec<Segment>,
    pub(crate) metas: Vec<Arc<Meta>>,
    pub(crate) type_to_meta: HashMap<TypeId, usize>,
    pub(crate) bits_to_segment: HashMap<Bits, usize>,
}

#[derive(Clone)]
pub struct State;

impl Meta {
    pub(crate) fn new<T: 'static>(index: usize) -> Self {
        Self {
            identifier: TypeId::of::<T>(),
            name: type_name::<T>(),
            index,
            size: size_of::<T>(),
            allocate: |capacity| {
                let mut data = ManuallyDrop::new(Vec::<T>::with_capacity(capacity));
                data.as_mut_ptr().cast()
            },
            free: |pointer, count, capacity| unsafe {
                Vec::from_raw_parts(pointer.cast::<T>(), count, capacity);
            },
            copy: if size_of::<T>() > 0 {
                |source, target, count| unsafe {
                    let source = source.0.cast::<T>().add(source.1);
                    let target = target.0.cast::<T>().add(target.1);
                    copy(source, target, count);
                }
            } else {
                |_, _, _| {}
            },
            drop: if needs_drop::<T>() {
                |pointer, index, count| unsafe {
                    let pointer = pointer.cast::<T>().add(index);
                    drop_in_place(slice_from_raw_parts_mut(pointer, count));
                }
            } else {
                |_, _, _| {}
            },
            set: if size_of::<T>() > 0 {
                |pointer, item, index| unsafe {
                    *pointer.cast::<T>().add(index) = *item.downcast().unwrap();
                }
            } else {
                |_, _, _| {}
            },
        }
    }
}

impl Inject for &World {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, _: Context) -> Option<Self::State> {
        Some(State)
    }
}

impl<'a> Get<'a> for State {
    type Item = &'a World;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        world
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            identifier: Self::reserve(),
            version: 1,
            segments: Vec::new(),
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
            bits_to_segment: HashMap::new(),
        };
        world.get_or_add_meta::<Entity>();
        world
    }

    #[inline]
    pub fn reserve() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub const fn version(&self) -> usize {
        self.version
    }

    pub fn get_meta<T: 'static>(&self) -> Option<Arc<Meta>> {
        let key = TypeId::of::<T>();
        let index = *self.type_to_meta.get(&key)?;
        Some(self.metas[index].clone())
    }

    pub fn get_or_add_meta<T: 'static>(&mut self) -> Arc<Meta> {
        match self.get_meta::<T>() {
            Some(meta) => meta,
            None => self.add_meta::<T>(),
        }
    }

    pub fn get_metas_from_types(&self, types: &Bits) -> Vec<Arc<Meta>> {
        types
            .into_iter()
            .filter_map(|index| self.metas.get(index))
            .cloned()
            .collect()
    }

    pub fn get_segment_by_types(&self, types: &Bits) -> Option<&Segment> {
        self.bits_to_segment
            .get(types)
            .map(|&index| &self.segments[index])
    }

    pub fn get_segment_by_metas(&self, metas: &[Arc<Meta>]) -> Option<&Segment> {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_segment_by_types(&types)
    }

    pub fn add_segment_from_types(&mut self, types: &Bits) -> &mut Segment {
        self.add_segment(types, &self.get_metas_from_types(types))
    }

    pub fn add_segment_from_metas(&mut self, metas: &[Arc<Meta>]) -> &mut Segment {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.add_segment_from_types(&types)
    }

    pub fn get_or_add_segment_by_types(&mut self, types: &Bits) -> &mut Segment {
        match self
            .get_segment_by_types(types)
            .map(|segment| segment.index)
        {
            Some(index) => &mut self.segments[index],
            None => self.add_segment_from_types(types),
        }
    }

    pub fn get_or_add_segment_by_metas(&mut self, metas: &[Arc<Meta>]) -> &mut Segment {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_or_add_segment_by_types(&types)
    }

    pub(crate) fn initialize<T: Default + 'static>(
        &mut self,
        default: Option<T>,
    ) -> Option<(Arc<Store>, usize)> {
        let meta = self.get_or_add_meta::<T>();
        let segment = self.get_or_add_segment_by_metas(&[meta.clone()]);
        let store = segment.store(&meta)?;
        if segment.count == 0 {
            let (index, _) = segment.reserve(1);
            segment.resolve();
            unsafe { store.set(index, default.unwrap_or_default()) };
        }
        Some((store, segment.index))
    }

    fn add_meta<T: 'static>(&mut self) -> Arc<Meta> {
        let index = self.metas.len();
        let meta = Arc::new(Meta::new::<T>(index));
        self.metas.push(meta.clone());
        self.type_to_meta.insert(meta.identifier.clone(), index);
        self.version += 1;
        meta
    }

    fn add_segment(&mut self, types: &Bits, metas: &[Arc<Meta>]) -> &mut Segment {
        let index = self.segments.len();
        let segment = Segment::new(index, types.clone(), metas, 0);
        self.segments.push(segment);
        self.bits_to_segment.insert(types.clone(), index);
        self.version += 1;
        &mut self.segments[index]
    }
}

pub mod segment {
    use super::*;

    pub struct Segment {
        pub(crate) index: usize,
        pub(crate) count: usize,
        pub(crate) stores: Box<[Arc<Store>]>,
        reserved: AtomicUsize,
        types: Bits,
        capacity: usize,
    }

    impl Segment {
        pub(crate) fn new(index: usize, types: Bits, metas: &[Arc<Meta>], capacity: usize) -> Self {
            let stores = metas
                .into_iter()
                .map(|meta| Store::new(meta.clone(), capacity).into())
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
}

pub mod store {
    use super::*;

    pub struct Store(pub(crate) Arc<Meta>, pub(crate) UnsafeCell<*mut ()>);

    unsafe impl Sync for Store {}
    unsafe impl Send for Store {}

    impl Store {
        #[inline]
        pub fn new(meta: Arc<Meta>, capacity: usize) -> Self {
            let pointer = (meta.allocate)(capacity);
            Self(meta, pointer.into())
        }

        #[inline]
        pub fn meta(&self) -> &Meta {
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
}
