use self::{meta::*, segment::*, store::*};
use crate::{entity::Entity, error::Error, Result};
use entia_core::{utility::next_power_of_2, Flags, IntoFlags, Maybe, Wrap};
use std::{
    any::{type_name, TypeId},
    cell::UnsafeCell,
    cmp::min,
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    mem::{needs_drop, replace, size_of, ManuallyDrop},
    ptr::{copy, drop_in_place, slice_from_raw_parts_mut},
    slice::from_raw_parts_mut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub struct World {
    identifier: usize,
    version: usize,
    pub(crate) segments: Vec<Segment>,
    pub(crate) metas: Vec<Arc<Meta>>,
    pub(crate) type_to_meta: HashMap<TypeId, usize>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            identifier: Self::reserve(),
            version: 1,
            segments: Vec::new(),
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
        };

        // Ensures that the 'Entity' meta has the lowest identifier of this world's metas and as such, 'Entity' stores will alway
        // appear as the first store of a segment if present.
        crate::metas!(world, Entity);
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

    pub fn set_meta(&mut self, meta: Arc<Meta>) {
        println!(
            "{} | {} | {}",
            meta.name,
            meta.clone.is_some(),
            meta.format.is_some(),
        );
        let identifier = meta.identifier;
        match self.type_to_meta.get(&identifier) {
            Some(&index) => self.metas[index] = meta,
            None => {
                let index = self.metas.len();
                self.metas.push(meta);
                self.type_to_meta.insert(identifier, index);
            }
        };
        self.version += 1;
    }

    pub fn get_meta<T: Send + Sync + 'static>(&self) -> Result<Arc<Meta>> {
        let key = TypeId::of::<T>();
        let index = *self
            .type_to_meta
            .get(&key)
            .ok_or(Error::MissingMeta(type_name::<T>()))?;
        Ok(self.metas[index].clone())
    }

    pub fn get_or_add_meta<T: Send + Sync + 'static>(&mut self) -> Arc<Meta> {
        match self.get_meta::<T>() {
            Ok(meta) => meta,
            Err(_) => {
                let meta = Arc::new(Meta::new::<T>(None, None));
                self.set_meta(meta.clone());
                meta
            }
        }
    }

    pub fn get_segment(&self, metas: &[Arc<Meta>]) -> Option<&Segment> {
        let types: HashSet<_> = metas.iter().map(|meta| meta.identifier).collect();
        Some(&self.segments[self.get_segment_index(&types)?])
    }

    pub fn get_or_add_segment<'a>(&'a mut self, metas: &[Arc<Meta>]) -> &'a mut Segment {
        let types: HashSet<_> = metas.iter().map(|meta| meta.identifier).collect();
        let index = match self.get_segment_index(&types) {
            Some(index) => index,
            None => {
                let index = self.segments.len();
                let segment = Segment::new(index, 0, types, &self.metas);
                self.segments.push(segment);
                self.version += 1;
                index
            }
        };
        &mut self.segments[index]
    }

    pub(crate) fn initialize<T: Default + Send + Sync + 'static>(
        &mut self,
        default: Option<T>,
    ) -> Result<(Arc<Store>, usize)> {
        let meta = self.get_or_add_meta::<T>();
        let segment = self.get_or_add_segment(&[meta.clone()]);
        let store = segment.store(&meta)?;
        if segment.count() == 0 {
            let (index, _) = segment.reserve(1);
            segment.resolve();
            unsafe { store.set(index, default.unwrap_or_default()) };
        }
        Ok((store, segment.index()))
    }

    fn get_segment_index(&self, types: &HashSet<TypeId>) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| &segment.types == types)
    }
}

pub mod meta {
    use super::*;

    #[derive(Clone)]
    pub struct Meta {
        pub(crate) identifier: TypeId,
        pub(crate) name: &'static str,
        pub(crate) size: usize,
        pub(crate) allocate: fn(usize) -> *mut (),
        pub(crate) free: unsafe fn(*mut (), usize, usize),
        pub(crate) copy: unsafe fn((*const (), usize), (*mut (), usize), usize),
        pub(crate) drop: unsafe fn(*mut (), usize, usize),
        pub(crate) clone: Option<unsafe fn((*const (), usize), (*mut (), usize))>,
        pub(crate) format: Option<unsafe fn(*const (), usize) -> String>,
    }

    pub struct Cloner<T: ?Sized>(
        unsafe fn((*const (), usize), (*mut (), usize)),
        PhantomData<T>,
    );

    pub struct Formatter<T: ?Sized>(unsafe fn(*const (), usize) -> String, PhantomData<T>);

    impl Meta {
        pub fn new<T: Send + Sync + 'static>(
            cloner: Option<Cloner<T>>,
            formatter: Option<Formatter<T>>,
        ) -> Self {
            Self {
                identifier: TypeId::of::<T>(),
                name: type_name::<T>(),
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
                clone: cloner.map(|cloner| cloner.0),
                format: formatter.map(|formatter| formatter.0),
            }
        }
    }

    impl<T: Clone> Maybe<Cloner<T>> for Wrap<Cloner<T>> {
        fn maybe(self) -> Option<Cloner<T>> {
            Some(Cloner::new())
        }
    }

    impl<T: fmt::Debug> Maybe<Formatter<T>> for Wrap<Formatter<T>> {
        fn maybe(self) -> Option<Formatter<T>> {
            Some(Formatter::new())
        }
    }

    impl<T: Clone> Cloner<T> {
        pub fn new() -> Self {
            Self(
                |source, target| unsafe {
                    let source = &*source.0.cast::<T>().add(source.1);
                    let target = target.0.cast::<T>().add(target.1);
                    // Use 'ptd::write' to prevent the old value from being dropped since it might not be initialized.
                    target.write(source.clone());
                },
                PhantomData,
            )
        }
    }

    impl<T: fmt::Debug> Formatter<T> {
        pub fn new() -> Self {
            Self(
                |source, index| unsafe {
                    let source = &*source.cast::<T>().add(index);
                    format!("{:?}", source)
                },
                PhantomData,
            )
        }
    }

    #[macro_export]
    macro_rules! metas {
        ($world:expr $(,$types:ty)*) => {{
            use $crate::core::Maybe;
            $(
                $world.set_meta($crate::world::meta::Meta::new(
                    $crate::core::Wrap::<$crate::world::meta::Cloner<$types>>::default().maybe(),
                    $crate::core::Wrap::<$crate::world::meta::Formatter<$types>>::default().maybe(),
                ).into());
            )*
        }};
    }
}

pub mod segment {
    use super::*;

    #[derive(Clone, Copy)]
    pub enum Flag {
        None = 0,
        Clone = 1 << 0,
    }

    pub struct Segment {
        index: usize,
        count: usize,
        flags: Flags<Flag>,
        stores: Box<[Arc<Store>]>,
        pub(super) types: HashSet<TypeId>,
        reserved: AtomicUsize,
        capacity: usize,
    }

    impl Segment {
        pub(super) fn new(
            index: usize,
            capacity: usize,
            types: HashSet<TypeId>,
            metas: &[Arc<Meta>],
        ) -> Self {
            // Iterate over all metas in order to have them consistently ordered.
            let stores: Box<[_]> = metas
                .iter()
                .filter(|meta| types.contains(&meta.identifier))
                .map(|meta| Arc::new(Store::new(meta.clone(), capacity)))
                .collect();
            let mut flags = Flag::None.flags();
            if stores.iter().all(|store| store.meta().clone.is_some()) {
                flags |= Flag::Clone;
            }
            Self {
                index,
                count: 0,
                flags,
                types,
                stores,
                reserved: 0.into(),
                capacity: 0,
            }
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
        pub fn has_store(&self, identifier: &TypeId) -> bool {
            self.types.contains(identifier)
        }

        pub fn remove_at(&mut self, index: usize) -> bool {
            if index < self.count {
                self.count -= 1;
                if index == self.count {
                    for store in self.stores() {
                        unsafe { store.drop(index, 1) };
                    }
                    false
                } else {
                    for store in self.stores() {
                        unsafe { store.squash(self.count, index) };
                    }
                    true
                }
            } else {
                false
            }
        }

        pub fn clear(&mut self) {
            for store in self.stores() {
                unsafe { store.drop(0, self.count) };
            }
            self.count = 0;
        }

        pub fn stores(&self) -> impl ExactSizeIterator<Item = &Store> {
            self.stores.iter().map(AsRef::as_ref)
        }

        pub fn store_at(&self, index: usize) -> Arc<Store> {
            self.stores[index].clone()
        }

        pub fn store(&self, meta: &Meta) -> Result<Arc<Store>> {
            self.stores
                .iter()
                .filter(|store| store.meta().identifier == meta.identifier)
                .next()
                .cloned()
                .ok_or(Error::MissingStore(meta.name, self.index))
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
            for store in self.stores() {
                unsafe { (store.meta().free)(store.data(), self.count, self.capacity) };
            }
        }
    }

    impl IntoFlags for Flag {
        type Value = usize;

        fn flags(self) -> Flags<Self, Self::Value> {
            Flags::new(self as usize)
        }
    }
}

pub mod store {
    use super::*;

    pub struct Store(Arc<Meta>, pub(crate) UnsafeCell<*mut ()>);

    // SAFETY: 'Sync' and 'Send' can be implemented for 'Store' because the only way to get a 'Meta' for some type is through a
    // 'World' which ensures that the type is 'Send' and 'Sync'.
    unsafe impl Sync for Store {}
    unsafe impl Send for Store {}

    impl Store {
        #[inline]
        pub(super) fn new(meta: Arc<Meta>, capacity: usize) -> Self {
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

        /// SAFETY: The target must be dropped before calling this function.
        #[inline]
        pub unsafe fn clone(source: (&Self, usize), target: (&Self, usize)) -> Result {
            debug_assert_eq!(source.0.meta().identifier, target.0.meta().identifier);

            let metas = (source.0.meta(), target.0.meta());
            let error = Error::MissingClone(metas.0.name);
            let clone = metas.0.clone.or(metas.1.clone).ok_or(error)?;
            clone((source.0.data(), source.1), (target.0.data(), target.1));
            Ok(())
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
