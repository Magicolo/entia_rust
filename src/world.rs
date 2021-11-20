use self::{meta::*, segment::*, store::*};
use crate::{
    entity::Entity,
    error::{Error, Result},
};
use entia_core::{utility::next_power_of_2, Flags, IntoFlags, Maybe, Wrap};
use std::{
    any::{type_name, TypeId},
    cell::UnsafeCell,
    cmp::min,
    collections::{HashMap, HashSet},
    fmt,
    iter::once,
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
    metas: Vec<Arc<Meta>>,
    type_to_meta: HashMap<TypeId, usize>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) resources: HashMap<TypeId, Arc<Store>>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            identifier: Self::reserve(),
            version: 1,
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
            segments: Vec::new(),
            resources: HashMap::new(),
        };

        // Ensures that the 'Entity' meta has the lowest identifier of this world's metas and as such, 'Entity' stores will alway
        // appear as the first store of a segment if present.
        crate::metas!(world, Entity);
        world
    }

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
                let segment = Segment::new(Self::reserve(), index, 0, types, &self.metas);
                self.segments.push(segment);
                self.version += 1;
                index
            }
        };
        &mut self.segments[index]
    }

    pub(crate) fn get_or_add_resource_store<T: Send + Sync + 'static>(
        &mut self,
        default: impl FnOnce() -> T,
    ) -> Arc<Store> {
        let meta = self.get_or_add_meta::<T>();
        let identifier = meta.identifier;
        let store = match self.resources.get(&identifier) {
            Some(store) => store.clone(),
            None => {
                let store = Arc::new(Store::new(meta, 1));
                unsafe { store.set(0, default()) };
                self.resources.insert(identifier, store.clone());
                store
            }
        };
        store
    }

    fn get_segment_index(&self, types: &HashSet<TypeId>) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| segment.component_types() == types)
    }
}

impl Drop for World {
    fn drop(&mut self) {
        for (_, store) in &self.resources {
            unsafe { store.free(1, 1) };
        }
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
        pub(crate) cloner: Option<Cloner>,
        pub(crate) formatter: Option<Formatter>,
    }

    #[derive(Copy, Clone)]
    pub struct Cloner<T: ?Sized = ()> {
        pub(crate) clone: unsafe fn((*const (), usize), (*mut (), usize), usize),
        pub(crate) duplicate: unsafe fn((*const (), usize), (*mut (), usize), usize),
        _marker: PhantomData<T>,
    }

    #[derive(Copy, Clone)]
    pub struct Formatter<T: ?Sized = ()> {
        pub(crate) format: unsafe fn(*const (), usize) -> String,
        _marker: PhantomData<T>,
    }

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
                    let mut pointer = ManuallyDrop::new(Vec::<T>::with_capacity(capacity));
                    pointer.as_mut_ptr().cast()
                },
                free: |pointer, count, capacity| unsafe {
                    Vec::from_raw_parts(pointer.cast::<T>(), count, capacity);
                },
                copy: if size_of::<T>() > 0 {
                    |source, target, count| unsafe {
                        if count > 0 {
                            let source = source.0.cast::<T>().add(source.1);
                            let target = target.0.cast::<T>().add(target.1);
                            copy(source, target, count);
                        }
                    }
                } else {
                    |_, _, _| {}
                },
                drop: if needs_drop::<T>() {
                    |pointer, index, count| unsafe {
                        if count > 0 {
                            let pointer = pointer.cast::<T>().add(index);
                            drop_in_place(slice_from_raw_parts_mut(pointer, count));
                        }
                    }
                } else {
                    |_, _, _| {}
                },
                cloner: cloner.map(Cloner::discard),
                formatter: formatter.map(Formatter::discard),
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

    impl<T> Cloner<T> {
        pub(crate) fn discard(self) -> Cloner {
            Cloner {
                clone: self.clone,
                duplicate: self.duplicate,
                _marker: PhantomData,
            }
        }
    }

    impl<T: Clone> Cloner<T> {
        pub fn new() -> Self {
            Self {
                clone: |source, target, count| unsafe {
                    if count > 0 {
                        let source = source.0.cast::<T>().add(source.1);
                        let target = target.0.cast::<T>().add(target.1);
                        // Use 'ptd::write' to prevent the old value from being dropped since it might not be initialized.
                        for i in 0..count {
                            let source = &*source.add(i);
                            target.add(i).write(source.clone());
                        }
                    }
                },
                duplicate: |source, target, count| unsafe {
                    if count > 0 {
                        let source = &*source.0.cast::<T>().add(source.1);
                        let target = target.0.cast::<T>().add(target.1);
                        // Use 'ptd::write' to prevent the old value from being dropped since it might not be initialized.
                        for i in 0..count {
                            target.add(i).write(source.clone());
                        }
                    }
                },
                _marker: PhantomData,
            }
        }
    }

    impl<T> Formatter<T> {
        pub fn discard(self) -> Formatter {
            Formatter {
                format: self.format,
                _marker: PhantomData,
            }
        }
    }

    impl<T: fmt::Debug> Formatter<T> {
        pub fn new() -> Self {
            Self {
                format: |source, index| unsafe {
                    let source = &*source.cast::<T>().add(index);
                    format!("{:?}", source)
                },
                _marker: PhantomData,
            }
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

    // The 'entities' store must be kept separate from the  'components' stores to prevent undesired behavior that may arise
    // from using queries such as '&mut Entity' or templates such as 'Add<Entity>' as a component with a template.
    pub struct Segment {
        identifier: usize,
        index: usize,
        count: usize,
        flags: Flags<Flag>,
        entity_store: Arc<Store>,
        component_stores: Vec<Arc<Store>>,
        component_types: HashSet<TypeId>,
        reserved: AtomicUsize,
        capacity: usize,
    }

    impl Segment {
        pub(super) fn new(
            identifier: usize,
            index: usize,
            capacity: usize,
            types: HashSet<TypeId>,
            metas: &[Arc<Meta>],
        ) -> Self {
            let entity_store = metas
                .iter()
                .find_map(|meta| {
                    if meta.identifier == TypeId::of::<Entity>() {
                        Some(Arc::new(Store::new(meta.clone(), capacity)))
                    } else {
                        None
                    }
                })
                .expect("Entity meta must be included.");

            // Iterate over all metas in order to have them consistently ordered.
            let component_stores: Vec<_> = metas
                .iter()
                .filter(|meta| types.contains(&meta.identifier))
                .map(|meta| Arc::new(Store::new(meta.clone(), capacity)))
                .collect();
            let mut flags = Flag::None.flags();
            if component_stores
                .iter()
                .all(|store| store.meta().cloner.is_some())
            {
                flags |= Flag::Clone;
            }

            Self {
                identifier,
                index,
                count: 0,
                flags,
                component_types: types,
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
            self.component_stores
                .iter()
                .find(|store| store.meta().identifier == meta.identifier)
                .cloned()
                .ok_or(Error::MissingStore(meta.name, self.index))
        }

        pub fn stores(&self) -> impl Iterator<Item = &Store> {
            once(self.entity_store.as_ref()).chain(self.component_stores())
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
            self.count += replace(reserved, 0);

            if self.capacity < count {
                let capacity = next_power_of_2(count as u32 - 1) as usize;
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
}

pub mod store {
    use super::*;

    pub struct Store(pub(super) Arc<Meta>, UnsafeCell<*mut ()>);

    // SAFETY: 'Sync' and 'Send' can be implemented for 'Store' because the only way to get a 'Meta' for some type is through a
    // 'World' which ensures that the type is 'Send' and 'Sync'.
    unsafe impl Sync for Store {}
    unsafe impl Send for Store {}

    impl Store {
        pub(super) fn new(meta: Arc<Meta>, capacity: usize) -> Self {
            let pointer = (meta.allocate)(capacity);
            Self(meta, pointer.into())
        }

        #[inline]
        pub fn meta(&self) -> &Meta {
            &self.0
        }

        #[inline]
        pub fn data<T: 'static>(&self) -> *mut T {
            debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier);
            self.pointer().cast()
        }

        #[inline]
        pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
            debug_assert_eq!(source.0.meta().identifier, target.0.meta().identifier);
            (source.0.meta().copy)(
                (source.0.pointer(), source.1),
                (target.0.pointer(), target.1),
                count,
            );
        }

        /// SAFETY: The target must be dropped before calling this function.
        #[inline]
        pub unsafe fn clone(
            source: (&Self, usize),
            target: (&Self, usize),
            count: usize,
        ) -> Result {
            debug_assert_eq!(source.0.meta().identifier, target.0.meta().identifier);
            let metas = (source.0.meta(), target.0.meta());
            let error = Error::MissingClone(metas.0.name);
            let cloner = metas.0.cloner.or(metas.1.cloner).ok_or(error)?;
            (cloner.clone)(
                (source.0.pointer(), source.1),
                (target.0.pointer(), target.1),
                count,
            );
            Ok(())
        }

        /// SAFETY: The target must be dropped before calling this function.
        #[inline]
        pub unsafe fn duplicate(
            source: (&Self, usize),
            target: (&Self, usize),
            count: usize,
        ) -> Result {
            debug_assert_eq!(source.0.meta().identifier, target.0.meta().identifier);
            let metas = (source.0.meta(), target.0.meta());
            let error = Error::MissingClone(metas.0.name);
            let cloner = metas.0.cloner.or(metas.1.cloner).ok_or(error)?;
            (cloner.duplicate)(
                (source.0.pointer(), source.1),
                (target.0.pointer(), target.1),
                count,
            );
            Ok(())
        }

        pub unsafe fn chunk(&self, index: usize, count: usize) -> Result<Self> {
            debug_assert!(self.meta().cloner.is_some());
            let store = Self::new(self.0.clone(), count);
            Self::clone((self, index), (&store, 0), count)?;
            Ok(store)
        }

        /// SAFETY: The 'index' must be within the bounds of the store.
        #[inline]
        pub unsafe fn get<T: 'static>(&self, index: usize) -> &mut T {
            &mut *self.data::<T>().add(index)
        }

        /// SAFETY: Both 'index' and 'count' must be within the bounds of the store.
        #[inline]
        pub unsafe fn get_all<T: 'static>(&self, index: usize, count: usize) -> &mut [T] {
            from_raw_parts_mut(self.data::<T>().add(index), count)
        }

        #[inline]
        pub unsafe fn set<T: 'static>(&self, index: usize, item: T) {
            self.data::<T>().add(index).write(item);
        }

        #[inline]
        pub unsafe fn set_all<T: 'static>(&self, index: usize, items: &[T])
        where
            T: Copy,
        {
            let source = items.as_ptr().cast::<T>();
            let target = self.data::<T>().add(index);
            source.copy_to_nonoverlapping(target, items.len());
        }

        /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
        #[inline]
        pub unsafe fn squash(&self, source_index: usize, target_index: usize) {
            let meta = self.meta();
            let pointer = self.pointer();
            (meta.drop)(pointer, target_index, 1);
            (meta.copy)((pointer, source_index), (pointer, target_index), 1);
        }

        #[inline]
        pub unsafe fn drop(&self, index: usize, count: usize) {
            (self.meta().drop)(self.pointer(), index, count);
        }

        #[inline]
        pub unsafe fn free(&self, count: usize, capacity: usize) {
            (self.meta().free)(self.pointer(), count, capacity);
        }

        pub unsafe fn resize(&self, old_capacity: usize, new_capacity: usize) {
            let meta = self.meta();
            let old_pointer = self.pointer();
            let new_pointer = (self.meta().allocate)(new_capacity);
            (meta.copy)((old_pointer, 0), (new_pointer, 0), old_capacity);
            (meta.free)(old_pointer, 0, old_capacity);
            *self.1.get() = new_pointer;
        }

        #[inline]
        fn pointer(&self) -> *mut () {
            unsafe { *self.1.get() }
        }
    }
}
