use self::{meta::*, segment::*, store::*};
use crate::{
    entity::Entity,
    error::{Error, Result},
};
use entia_core::{utility::next_power_of_2, Flags, IntoFlags, Maybe, Wrap};
use std::{
    any::{type_name, Any, TypeId},
    cell::Cell,
    cmp::min,
    collections::{HashMap, HashSet},
    fmt,
    iter::once,
    mem::{needs_drop, replace, size_of, ManuallyDrop, MaybeUninit},
    ptr::{copy, drop_in_place, slice_from_raw_parts_mut},
    slice::from_raw_parts_mut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// Such a 'Link' would allow to compute which components have been added or removed.
/*
- Add 'Added/Removed<T>' query filters. The filters would hold a 'Bits' that represent the indices:
    fn dynamic_filter(state: &mut Self::State, index: usize) -> bool {
        state.bits.set(index, false) ????
    }
    - Will be equivalent to receiving a 'OnAdd<T>' message and 'query.get(onAdd.entity)'.
*/
// enum Link {
//     None,
//     Add { meta: usize, segment: usize },
//     Remove { meta: usize, segment: usize },
// }

pub struct World {
    identifier: usize,
    version: usize,
    metas: Vec<Arc<Meta>>,
    type_to_meta: HashMap<TypeId, usize>,
    resources: HashMap<TypeId, Arc<Store>>,
    // SAFETY: This vector may only 'push', never 'pop'; otherwise some unsafe index access may become invalid.
    segments: Vec<Segment>,
}

pub trait Component: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}

pub trait Resource: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }

    fn initialize(meta: &Meta, _: &mut World) -> Result<Self> {
        match meta.default() {
            Some(resource) => Ok(resource),
            None => Err(Error::MissingResource {
                name: type_name::<Self>(),
                identifier: TypeId::of::<Self>(),
            }),
        }
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            identifier: Self::reserve(),
            version: 1,
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
            resources: HashMap::new(),
            segments: Vec::new(),
        };

        // Ensures that the 'Entity' meta has the lowest identifier of this world's metas and as such, 'Entity' stores will alway
        // appear as the first store of a segment if present.
        world.set_meta(crate::meta!(Entity));
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

    #[inline]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    #[inline]
    pub fn segments_mut(&mut self) -> &mut [Segment] {
        &mut self.segments
    }

    pub fn set_meta(&mut self, meta: Meta) -> Arc<Meta> {
        let identifier = meta.identifier();
        let meta = match self.type_to_meta.get(&identifier) {
            Some(&index) => {
                let meta = Arc::new(meta);
                self.metas[index] = meta.clone();
                meta
            }
            None => {
                let index = self.metas.len();
                let meta = Arc::new(meta);
                self.metas.push(meta.clone());
                self.type_to_meta.insert(meta.identifier(), index);
                meta
            }
        };
        self.version += 1;
        meta
    }

    pub fn get_meta<T: Send + Sync + 'static>(&self) -> Result<Arc<Meta>> {
        let key = TypeId::of::<T>();
        match self.type_to_meta.get(&key) {
            Some(&index) => Ok(self.metas[index].clone()),
            None => Err(Error::MissingMeta {
                name: type_name::<T>(),
                identifier: key,
            }),
        }
    }

    pub fn get_or_add_meta<T: Send + Sync + 'static, M: FnOnce() -> Meta>(
        &mut self,
        meta: M,
    ) -> Arc<Meta> {
        match self.get_meta::<T>() {
            Ok(meta) => meta,
            Err(_) => self.set_meta(meta()),
        }
    }

    pub fn get_segment(&self, metas: impl IntoIterator<Item = TypeId>) -> Option<&Segment> {
        Some(&self.segments[self.get_segment_index(&metas.into_iter().collect())?])
    }

    pub fn get_or_add_segment<'a>(
        &'a mut self,
        metas: impl IntoIterator<Item = Arc<Meta>>,
    ) -> &'a mut Segment {
        let types: HashSet<_> = metas.into_iter().map(|meta| meta.identifier()).collect();
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

    pub(crate) fn get_or_add_resource_store<
        T: Send + Sync + 'static,
        M: FnOnce() -> Meta,
        I: FnOnce(&Meta, &mut World) -> Result<T>,
    >(
        &mut self,
        meta: M,
        initialize: I,
    ) -> Result<Arc<Store>> {
        let meta = self.get_or_add_meta::<T, _>(meta);
        let identifier = meta.identifier();
        match self.resources.get(&identifier) {
            Some(store) => Ok(store.clone()),
            None => {
                let resource = initialize(&meta, self)?;
                let store = Arc::new(Store::new(meta, 1));
                unsafe { store.set(0, resource) };
                self.resources.insert(identifier, store.clone());
                Ok(store)
            }
        }
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

    type Module = dyn Any + Send + Sync;

    pub struct Meta {
        identifier: TypeId,
        name: &'static str,
        pub(super) allocate: fn(usize) -> *mut (),
        pub(super) free: unsafe fn(*mut (), usize, usize),
        pub(super) copy: unsafe fn((*const (), usize), (*mut (), usize), usize),
        pub(super) drop: unsafe fn(*mut (), usize, usize),
        pub(super) defaulter: Option<Defaulter>,
        pub(super) cloner: Option<Cloner>,
        pub(super) formatter: Option<Formatter>,
        modules: HashMap<TypeId, Box<Module>>,
    }

    #[derive(Clone)]
    pub struct Defaulter {
        pub default: unsafe fn(target: (*mut (), usize), count: usize),
    }

    #[derive(Clone)]
    pub struct Cloner {
        pub clone: unsafe fn(source: (*const (), usize), target: (*mut (), usize), count: usize),
        pub fill: unsafe fn(source: (*const (), usize), target: (*mut (), usize), count: usize),
    }

    #[derive(Clone)]
    pub struct Formatter {
        pub format: unsafe fn(source: *const (), index: usize) -> String,
    }

    impl Meta {
        // To increase safe usage of 'Meta' and 'Store', type 'T' is required to be 'Send' and 'Sync', therefore it is
        // impossible to hold an instance of 'Meta' that is not 'Send' and 'Sync'.
        pub fn new<T: Send + Sync + 'static, I: IntoIterator<Item = Box<Module>>>(
            modules: I,
        ) -> Self {
            let mut meta = Self {
                identifier: TypeId::of::<T>(),
                name: type_name::<T>(),
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
                defaulter: None,
                cloner: None,
                formatter: None,
                modules: modules
                    .into_iter()
                    .map(|module| (module.type_id(), module))
                    .collect(),
            };
            meta.reset();
            meta
        }

        #[inline]
        pub const fn identifier(&self) -> TypeId {
            self.identifier
        }

        #[inline]
        pub const fn name(&self) -> &'static str {
            self.name
        }

        pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
            self.modules
                .get(&TypeId::of::<T>())
                .and_then(|module| module.downcast_ref::<T>())
        }

        pub fn set<T: Send + Sync + 'static>(&mut self, module: T) {
            let module: Box<Module> = Box::new(module);
            self.modules.insert(TypeId::of::<T>(), module);
            self.reset();
        }

        pub fn default<T: 'static>(&self) -> Option<T> {
            if TypeId::of::<T>() == self.identifier {
                let defaulter = self.defaulter.as_ref()?;
                Some(unsafe {
                    let mut target = MaybeUninit::<T>::uninit();
                    (defaulter.default)((target.as_mut_ptr() as _, 0), 1);
                    target.assume_init()
                })
            } else {
                None
            }
        }

        pub fn clone<T: 'static>(&self, value: &T) -> Option<T> {
            if TypeId::of::<T>() == self.identifier {
                let cloner = self.cloner.as_ref()?;
                Some(unsafe {
                    let source = value as *const _ as _;
                    let mut target = MaybeUninit::<T>::uninit();
                    (cloner.clone)((source, 0), (target.as_mut_ptr() as _, 0), 1);
                    target.assume_init()
                })
            } else {
                None
            }
        }

        pub fn format<T: 'static>(&self, value: &T) -> Option<String> {
            if TypeId::of::<T>() == self.identifier {
                let formatter = self.formatter.as_ref()?;
                Some(unsafe {
                    let source = value as *const _ as _;
                    (formatter.format)(source, 0)
                })
            } else {
                None
            }
        }

        fn reset(&mut self) {
            self.defaulter = self.get().cloned();
            self.cloner = self.get().cloned();
            self.formatter = self.get().cloned();
        }
    }

    impl Defaulter {
        pub fn new<T: Default>() -> Self {
            Self {
                default: |target, count| unsafe {
                    let target = target.0.cast::<T>().add(target.1);
                    for i in 0..count {
                        target.add(i).write(T::default());
                    }
                },
            }
        }
    }

    impl<T: Default> Maybe<Defaulter> for Wrap<Defaulter, T> {
        fn maybe(self) -> Option<Defaulter> {
            Some(Defaulter::new::<T>())
        }
    }

    impl Cloner {
        pub fn new<T: Clone>() -> Self {
            Self {
                clone: if size_of::<T>() > 0 {
                    |source, target, count| unsafe {
                        let source = source.0.cast::<T>().add(source.1);
                        let target = target.0.cast::<T>().add(target.1);
                        // Use 'ptd::write' to prevent the old value from being dropped since it is expected to be already
                        // dropped or uninitialized.
                        for i in 0..count {
                            let source = &*source.add(i);
                            target.add(i).write(source.clone());
                        }
                    }
                } else {
                    // TODO: What about implementations of 'Clone' that have side-effects?
                    |_, _, _| {}
                },
                fill: if size_of::<T>() > 0 {
                    |source, target, count| unsafe {
                        let source = &*source.0.cast::<T>().add(source.1);
                        let target = target.0.cast::<T>().add(target.1);
                        // Use 'ptd::write' to prevent the old value from being dropped since it is expected to be already
                        // dropped or uninitialized.
                        for i in 0..count {
                            target.add(i).write(source.clone());
                        }
                    }
                } else {
                    // TODO: What about implementations of 'Clone' that have side-effects?
                    |_, _, _| {}
                },
            }
        }
    }

    impl<T: Clone> Maybe<Cloner> for Wrap<Cloner, T> {
        fn maybe(self) -> Option<Cloner> {
            Some(Cloner::new::<T>())
        }
    }

    impl Formatter {
        pub fn new<T: fmt::Debug>() -> Self {
            Self {
                format: |source, index| unsafe { format!("{:?}", &*source.cast::<T>().add(index)) },
            }
        }
    }

    impl<T: fmt::Debug> Maybe<Formatter> for Wrap<Formatter, T> {
        fn maybe(self) -> Option<Formatter> {
            Some(Formatter::new::<T>())
        }
    }

    #[macro_export]
    macro_rules! meta {
        ($t:ty) => {{
            use $crate::core::Maybe;

            let mut modules: std::vec::Vec<
                Box<dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static>,
            > = std::vec::Vec::new();

            type Defaulter<T> = $crate::core::Wrap<$crate::world::meta::Defaulter, T>;
            if let Some(module) = Defaulter::<$t>::default().maybe() {
                modules.push(std::boxed::Box::new(module));
            }
            type Cloner<T> = $crate::core::Wrap<$crate::world::meta::Cloner, T>;
            if let Some(module) = Cloner::<$t>::default().maybe() {
                modules.push(std::boxed::Box::new(module));
            }
            type Formatter<T> = $crate::core::Wrap<$crate::world::meta::Formatter, T>;
            if let Some(module) = Formatter::<$t>::default().maybe() {
                modules.push(std::boxed::Box::new(module));
            }

            $crate::world::meta::Meta::new::<$t, _>(modules)
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

    // The 'entities' store must be kept separate from the 'components' stores to prevent undesired behavior that may arise
    // from using queries such as '&mut Entity' or templates such as 'Add<Entity>' as a component with a template.
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
            identifier: usize,
            index: usize,
            capacity: usize,
            types: HashSet<TypeId>,
            metas: &[Arc<Meta>],
        ) -> Self {
            let entity_store = metas
                .iter()
                .find_map(|meta| {
                    if meta.identifier() == TypeId::of::<Entity>() {
                        Some(Arc::new(Store::new(meta.clone(), capacity)))
                    } else {
                        None
                    }
                })
                .expect("Entity meta must be included.");

            // Iterate over all metas in order to have them consistently ordered.
            let component_stores: Box<_> = metas
                .iter()
                .filter(|meta| types.contains(&meta.identifier()))
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
                (index, self.capacity - min(index, self.capacity))
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
}

pub mod store {
    use super::*;

    pub struct Store(Arc<Meta>, Cell<*mut ()>);

    // SAFETY: 'Sync' and 'Send' can be implemented for 'Store' because this crate ensures its proper usage. Other users
    // of this type must fulfill the safety requirements of its unsafe methods.
    unsafe impl Sync for Store {}
    unsafe impl Send for Store {}

    impl Store {
        pub(crate) fn new(meta: Arc<Meta>, capacity: usize) -> Self {
            let pointer = (meta.allocate)(capacity);
            Self(meta, pointer.into())
        }

        #[inline]
        pub fn meta(&self) -> &Meta {
            &self.0
        }

        /// In order to be consistent with the requirements of 'Meta::new', 'T' is required to be 'Send + Sync'.
        #[inline]
        pub fn data<T: Send + Sync + 'static>(&self) -> *mut T {
            debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier());
            self.1.get().cast()
        }

        #[inline]
        pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
            debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
            (source.0.meta().copy)(
                (source.0 .1.get(), source.1),
                (target.0 .1.get(), target.1),
                count,
            );
        }

        /// SAFETY: The target must be dropped before calling this function.
        pub unsafe fn clone(
            source: (&Self, usize),
            target: (&Self, usize),
            count: usize,
        ) -> Result {
            debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
            let metas = (source.0.meta(), target.0.meta());
            let error = Error::MissingClone {
                name: metas.0.name(),
            };
            let cloner = metas
                .0
                .cloner
                .as_ref()
                .or(metas.1.cloner.as_ref())
                .ok_or(error)?;
            (cloner.clone)(
                (source.0 .1.get(), source.1),
                (target.0 .1.get(), target.1),
                count,
            );
            Ok(())
        }

        /// SAFETY: The target must be dropped before calling this function.
        pub unsafe fn fill(source: (&Self, usize), target: (&Self, usize), count: usize) -> Result {
            debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
            let metas = (source.0.meta(), target.0.meta());
            let error = Error::MissingClone {
                name: metas.0.name(),
            };
            let cloner = metas
                .0
                .cloner
                .as_ref()
                .or(metas.1.cloner.as_ref())
                .ok_or(error)?;
            (cloner.fill)(
                (source.0 .1.get(), source.1),
                (target.0 .1.get(), target.1),
                count,
            );
            Ok(())
        }

        #[inline]
        pub unsafe fn chunk(&self, index: usize, count: usize) -> Result<Self> {
            debug_assert!(self.meta().cloner.is_some());
            let store = Self::new(self.0.clone(), count);
            Self::clone((self, index), (&store, 0), count)?;
            Ok(store)
        }

        /// SAFETY: The 'index' must be within the bounds of the store.
        #[inline]
        pub unsafe fn get<T: Send + Sync + 'static>(&self, index: usize) -> &mut T {
            &mut *self.data::<T>().add(index)
        }

        /// SAFETY: Both 'index' and 'count' must be within the bounds of the store.
        #[inline]
        pub unsafe fn get_all<T: Send + Sync + 'static>(
            &self,
            index: usize,
            count: usize,
        ) -> &mut [T] {
            from_raw_parts_mut(self.data::<T>().add(index), count)
        }

        #[inline]
        pub unsafe fn set<T: Send + Sync + 'static>(&self, index: usize, item: T) {
            self.data::<T>().add(index).write(item);
        }

        #[inline]
        pub unsafe fn set_all<T: Send + Sync + 'static>(&self, index: usize, items: &[T])
        where
            T: Copy,
        {
            let source = items.as_ptr().cast::<T>();
            let target = self.data::<T>().add(index);
            source.copy_to_nonoverlapping(target, items.len());
        }

        /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
        /// The ranges 'source_index..source_index + count' and 'target_index..target_index + count' must not overlap.
        #[inline]
        pub unsafe fn squash(&self, source_index: usize, target_index: usize, count: usize) {
            let meta = self.meta();
            let pointer = self.1.get();
            (meta.drop)(pointer, target_index, count);
            (meta.copy)((pointer, source_index), (pointer, target_index), count);
        }

        #[inline]
        pub unsafe fn drop(&self, index: usize, count: usize) {
            (self.meta().drop)(self.1.get(), index, count);
        }

        #[inline]
        pub unsafe fn free(&self, count: usize, capacity: usize) {
            (self.meta().free)(self.1.get(), count, capacity);
        }

        pub unsafe fn resize(&self, old_capacity: usize, new_capacity: usize) {
            let meta = self.meta();
            let old_pointer = self.1.get();
            let new_pointer = (self.meta().allocate)(new_capacity);
            (meta.copy)((old_pointer, 0), (new_pointer, 0), old_capacity);
            (meta.free)(old_pointer, 0, old_capacity);
            self.1.set(new_pointer);
        }
    }
}
