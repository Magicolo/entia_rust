#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(never_type)]
#![feature(strict_provenance_atomic_ptr)]
#![feature(generators)]
#![feature(generator_trait)]
#![feature(iter_from_generator)]

pub mod add;
pub mod component;
pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod entities;
pub mod entity;
pub mod error;
pub mod families;
pub mod family;
pub mod filter;
pub mod inject;
pub mod item;
pub mod message;
pub mod meta;
pub mod output;
pub mod query;
pub mod resource;
pub mod resources;
pub mod run;
pub mod schedule;
pub mod segment;
pub mod store;
pub mod system;
pub mod template;
pub mod world;

/*
- Remove `Entities` and store `Datum` in `Segments`.
    - `Entity` would become `{ generation: u32, segment: u32, store: u32 }`
    - `Datum` would become `{
        generation: u32,
        children: u32,
        parent: { segment: u32, store: u32 },
        first_child: { segment: u32, store: u32 },
        last_child: { segment: u32, store: u32 },
        previous_sibling: { segment: u32, store: u32 },
        next_sibling: { segment: u32, store: u32 },
    }` // 48 bytes
    - Would allow to remove the conflict betweem `Create::resolve` (`Write::<Entities>`) and `Query` (`Read::<Entities>`) in most cases.
    - Would allow to remove some conflicts between `Adopt`, `Reject`, `Destroy` as `Item` and `Create` as `Inject`.

- With the chunks iterators, it could be possible to add chunk operations such as 'Destroy/Add/Remove/Adopt/Reject'.

- When possible, decompose systems into smaller systems to allow more parallelism:
    - 'Query systems' may be divided in 'Chunk systems'.
        - Maybe 'Segment systems' would be a good enough approximation to split a system.
        - These systems must have no other purpose other than iterating a query, therefore their item dependencies do not overlap.

- Add a 'Plan<I: Inject, O: IntoOutput, F: FnMut(I) -> O, N = 1> { queues: &mut [Queue<Plan<I>>], last: usize }' injectable.
    - Allows to schedule runs dynamically based on the static dependencies of 'I'.
    - The 'N' parameter is the degree of parallelism and concretely, is the amount of runs that 'Plan' will schedule,.
    - The system that injects 'Plan' would not itself depend on 'I::depend' but rather on a 'Write<{ plan.identifier() }>'.
    - The system would be allowed to enqueue operations
    - Then, 'Plan' would schedule 'N' runs with 'I::depend' and 'Read<{ plan.identifier() }>'.
    - Each run would be responsible to empty a queue of 'Plan'.
    - To the eyes of the 'Scheduler', the 'N' runs are assumed to always be populated and thus, will block execution if their dependencies collide.
    - It may be possible to plan multiple query segment runs with 'plan.query'.
    /*
        |physics: &Physics, mut a: Plan<_, _, _>|, mut b: Plan<_, _, _> {
            if physics.gravity.y > 0 {
                // Resolves 'planA' to 'Plan<Query<&mut Mass>, (), _>'.
                planA.query(|mass: &mut Mass| mass.disable = true);
            } else {
                // Resolves 'planA' to 'Plan<Query<&mut Mass>, (), _>'.
                planB.query_chunks(|position: &mut [Position]| position.x += 1);
            }
        }
    */

- For 'Create', resolution of 'Entities' and segments may be achievable in parallel:
    - Deferral would be grouped by segment.
    - Each parallel application (copy of data) of a segment has dependencies [
        'Read<Entities>' (to ensure ordering with the resolution of 'Entities'),
        'Write<Entity>' for the segment,
    ]
    - When to resolve 'Entities'?
*/

pub mod core {
    pub use entia_core::*;
}

use std::sync::atomic::{AtomicUsize, Ordering};

pub use crate::{
    component::Component,
    create::Create,
    defer::Defer,
    destroy::Destroy,
    entity::Entity,
    families::{adopt::Adopt, reject::Reject, Families},
    family::Family,
    filter::{Filter, Has, Not},
    inject::{Inject, Injector},
    message::{emit::Emit, receive::Receive, Message},
    query::Query,
    resource::Resource,
    run::Runner,
    schedule::Scheduler,
    system::{IntoSystem, System},
    template::{Add, LeafTemplate, Spawn, SpawnTemplate, StaticTemplate, Template, With},
    world::World,
};
pub(crate) use entia_macro::{tuples_16 as tuples, tuples_with_16 as tuples_with};
pub use entia_main_derive::{Component, Filter, Message, Resource, Template};

pub fn identify() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod test;

pub mod database {
    /*
    COHERENCE RULES:
    - Legend:
        ->: Left happens before right.
        <->: Left before or after right.
        <-: Left happens after right.
        -^: Left is used before and resolved after right.
        <-^: Left is used before or after and resolved after right.

    - `Query` -> `Create`: `Query` must read the count of its overlapping tables before the `Create` begins.
    - `Create` -> `Query`: `Query` must wait for the end of `Create` before reading/writing its overlapping tables.
    - `Defer<Create>` <-^ `Query`: `Query` must read the count of its overlapping tables before the `Defer<Create>` is resolved.
    - `Create` -> `Destroy`:

    TODO: There is no need to take a `Table` lock when querying as long as the store locks are always taken from left to right.
    TODO: Implement drop for `Inner` and `Table`.

    - `Defer<Create>` can do most of the work of `Create` without making the changes observable.
        - Only initializing the slots and resolving the table count need to be deferred.
        - The table then must go into a state where any `Destroy` operations must consider the `reserved` count of the table when doing
        the swap and the resolution of `Create` must consider that it may have been moved or destroyed (maybe `slot.generation` can help?).

    Design an ergonomic and massively parallel database with granular locking and great cache locality.
    - Most of the locking would be accomplished with `parking_lot::RWLock`.
    - Locking could be as granular as a single field within a `struct`.
        - Ex: Writing to `position.y` could require a read lock on the `Position` store and a write lock on `y`.
        This way, `Position` can still be read or written to by other threads.
    - All accessors to the database that use locks can safely modify it immediately. The lock free accessors will defer their operations.
    - A defer accessor will provide its 2 parts: `Defer` and `Resolve`.
        - They will share a special access to some data in the database to allow `Resolve` to properly resolve the deferred operations.
        - The ordering of different deferred operations is currently unclear.
    */

    use entia_core::utility::next_power_of_2;
    use parking_lot::{
        MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard,
        RwLockUpgradableReadGuard, RwLockWriteGuard,
    };
    use std::{
        any::{Any, TypeId},
        cell::{RefCell, UnsafeCell},
        collections::{HashMap, VecDeque},
        iter::{from_fn, from_generator},
        marker::PhantomData,
        mem::{forget, replace, ManuallyDrop},
        ops::{Deref, DerefMut, Generator, GeneratorState},
        pin::Pin,
        ptr::{self, copy_nonoverlapping, NonNull},
        slice::{from_raw_parts, from_raw_parts_mut, SliceIndex},
        sync::{
            atomic::{AtomicI64, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering::*},
            Arc,
        },
        thread::yield_now,
    };

    pub struct Meta {
        identifier: TypeId,
    }

    pub(crate) struct Store {
        meta: Arc<Meta>,
        data: RwLock<NonNull<()>>,
    }

    pub struct Table {
        identifier: usize,
        count: AtomicU64,
        capacity: u32,
        indices: HashMap<TypeId, usize>,
        keys: Store,
        /// Stores must be ordered consistently between tables (ex: by the `TypeId` of each `store.meta`).
        stores: Box<[Store]>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Key {
        index: u32,
        generation: u32,
    }

    pub(crate) struct Slot {
        generation: AtomicU32,
        indices: AtomicU64,
    }

    pub(crate) struct Inner {
        free: RwLock<(Vec<Key>, AtomicI64)>,
        slots: (RwLock<u32>, UnsafeCell<Vec<Box<[Slot; Self::CHUNK]>>>),
        tables: Vec<RwLock<Table>>,
    }

    pub struct Database(Arc<Inner>);

    pub enum Error {}

    pub trait Datum: 'static {
        fn meta() -> Meta {
            todo!()
        }
    }

    pub trait Item {
        type State: for<'a> At<'a>;
        fn initialize(table: &Table) -> Option<Self::State>;
    }

    pub trait At<'a> {
        type State;
        type Chunk;
        type Item;

        fn get(&self, table: &Table) -> Option<Self::State>;
        unsafe fn chunk(state: &mut Self::State) -> Self::Chunk;
        unsafe fn item(state: &mut Self::State, index: usize) -> Self::Item;
    }

    pub unsafe trait Template {
        unsafe fn apply(self, store: usize, table: &Table);
    }

    pub struct Spawn<T: Template>(PhantomData<T>);
    pub struct With<T: Template, F: FnMut(Key) -> T>(F, PhantomData<T>);

    pub struct Read<C>(usize, PhantomData<C>);

    pub struct Create<T: Template> {
        _marker: PhantomData<T>,
    }

    pub struct Destroy {}

    pub struct Keys {}

    pub struct Query<I: Item, F: Filter = ()> {
        inner: Arc<Inner>,
        index: usize,
        indices: HashMap<usize, usize>,
        states: Vec<(usize, I::State)>,
        queue: VecDeque<usize>,
        filter: F,
        _marker: PhantomData<fn(I)>,
    }

    pub struct Guard<'a, T>(T, RwLockReadGuard<'a, Table>);

    pub trait Filter {
        fn filter(&mut self, table: &Table) -> bool;
    }
    pub struct Not<F: Filter>(F);
    pub struct Has<D: Datum>(PhantomData<D>);

    impl Filter for () {
        fn filter(&mut self, table: &Table) -> bool {
            todo!()
        }
    }
    impl<F: Filter> Filter for Not<F> {
        fn filter(&mut self, table: &Table) -> bool {
            !self.0.filter(table)
        }
    }
    impl<D: Datum> Filter for Has<D> {
        fn filter(&mut self, table: &Table) -> bool {
            table.indices.contains_key(&TypeId::of::<D>())
        }
    }
    impl<F: FnMut(&Table) -> bool> Filter for F {
        fn filter(&mut self, table: &Table) -> bool {
            self(table)
        }
    }

    impl Key {
        pub const NULL: Self = Self {
            index: u32::MAX,
            generation: u32::MAX,
        };

        #[inline]
        pub(crate) const fn new(index: u32) -> Self {
            Self {
                index: index,
                generation: 0,
            }
        }
    }

    impl Slot {
        const fn recompose_indices(table: u32, store: u32) -> u64 {
            ((table as u64) << 32) | (store as u64)
        }

        const fn decompose_indices(indices: u64) -> (u32, u32) {
            ((indices >> 32) as u32, indices as u32)
        }

        #[inline]
        pub fn new(table: u32, store: u32) -> Self {
            let indices = AtomicU64::new(Self::recompose_indices(table, store));
            Self {
                generation: 0.into(),
                indices,
            }
        }

        #[inline]
        pub fn initialize(&self, generation: u32, table: u32, store: u32) {
            self.generation.store(generation, Release);
            self.update(table, store);
        }

        #[inline]
        pub fn update(&self, table: u32, store: u32) {
            let indices = Self::recompose_indices(table, store);
            self.indices.store(indices, Release);
        }

        #[inline]
        pub fn release(&self, generation: u32) -> Option<(u32, u32)> {
            self.generation
                .compare_exchange(generation, u32::MAX, AcqRel, Acquire)
                .ok()?;
            let indices = self.indices.swap(u64::MAX, Release);
            debug_assert!(indices < u64::MAX);
            Some(Self::decompose_indices(indices))
        }

        #[inline]
        pub fn generation(&self) -> u32 {
            self.generation.load(Acquire)
        }

        #[inline]
        pub fn indices(&self) -> (u32, u32) {
            Self::decompose_indices(self.indices.load(Acquire))
        }

        #[inline]
        pub fn valid_with(&self, generation: u32) -> bool {
            self.generation() == generation
        }
    }

    impl Default for Slot {
        fn default() -> Self {
            Self::new(u32::MAX, u32::MAX)
        }
    }

    impl Store {
        #[inline]
        pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
            // debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
            // (source.0.meta().copy)(
            //     (source.0.data.get(), source.1),
            //     (target.0.data.get(), target.1),
            //     count,
            // );
        }

        pub unsafe fn grow(&self, old_capacity: usize, new_capacity: usize) {
            // debug_assert!(old_capacity < new_capacity);
            // let meta = self.meta();
            // let old_pointer = self.data.get();
            // let new_pointer = (self.meta().allocate)(new_capacity);
            // (meta.copy)((old_pointer, 0), (new_pointer, 0), old_capacity);
            // (meta.free)(old_pointer, 0, old_capacity);
            // self.data.set(new_pointer);
        }

        /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
        /// The ranges 'source_index..source_index + count' and 'target_index..target_index + count' must not overlap.
        #[inline]
        pub unsafe fn squash(&self, source_index: usize, target_index: usize, count: usize) {
            // let meta = self.meta();
            // let pointer = self.data.get();
            // (meta.drop)(pointer, target_index, count);
            // (meta.copy)((pointer, source_index), (pointer, target_index), count);
        }

        #[inline]
        pub unsafe fn drop(&self, index: usize, count: usize) {
            // (self.meta().drop)(self.data.get(), index, count);
        }

        #[inline]
        pub unsafe fn read<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> MappedRwLockReadGuard<I::Output> {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            RwLockReadGuard::map(self.data.read(), |data| unsafe {
                from_raw_parts(data.as_ptr().cast::<T>(), count).get_unchecked(index)
            })
        }

        #[inline]
        pub unsafe fn try_read<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> Option<MappedRwLockReadGuard<I::Output>> {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            let data = self.data.try_read()?;
            Some(RwLockReadGuard::map(data, |data| unsafe {
                from_raw_parts(data.as_ptr().cast::<T>(), count).get_unchecked(index)
            }))
        }

        #[inline]
        pub unsafe fn read_unlocked_at<T: 'static>(&self, index: usize) -> &T {
            self.read_unlocked(index, index + 1)
        }

        #[inline]
        pub unsafe fn read_unlocked<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> &I::Output {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            let data = *self.data.data_ptr();
            from_raw_parts(data.as_ptr().cast::<T>(), count).get_unchecked(index)
        }

        #[inline]
        pub unsafe fn write<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> MappedRwLockWriteGuard<I::Output> {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            RwLockWriteGuard::map(self.data.write(), |data| unsafe {
                from_raw_parts_mut(data.as_ptr().cast::<T>(), count).get_unchecked_mut(index)
            })
        }

        #[inline]
        pub unsafe fn write_at<T: 'static>(&self, index: usize) -> MappedRwLockWriteGuard<T> {
            self.write(index, index + 1)
        }

        #[inline]
        pub unsafe fn write_all<T: 'static>(&self, count: usize) -> MappedRwLockWriteGuard<[T]> {
            self.write(.., count)
        }

        #[inline]
        pub unsafe fn try_write<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> Option<MappedRwLockWriteGuard<I::Output>> {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            let data = self.data.try_write()?;
            Some(RwLockWriteGuard::map(data, |data| unsafe {
                from_raw_parts_mut(data.as_ptr().cast::<T>(), count).get_unchecked_mut(index)
            }))
        }

        #[inline]
        pub unsafe fn write_unlocked_at<T: 'static>(&self, index: usize) -> &mut T {
            self.write_unlocked(index, index + 1)
        }

        #[inline]
        pub unsafe fn write_unlocked<T: 'static, I: SliceIndex<[T]>>(
            &self,
            index: I,
            count: usize,
        ) -> &mut I::Output {
            debug_assert_eq!(TypeId::of::<T>(), self.meta.identifier);
            let data = *self.data.data_ptr();
            from_raw_parts_mut(data.as_ptr().cast::<T>(), count).get_unchecked_mut(index)
        }
    }

    impl Table {
        #[inline]
        pub fn count(&self) -> u32 {
            self.count.load(Acquire) as _
        }

        pub fn grow(&mut self, capacity: u32) {
            let capacity = next_power_of_2(capacity);
            unsafe { self.keys.grow(self.capacity as _, capacity as _) };
            for store in self.stores.iter() {
                unsafe { store.grow(self.capacity as _, capacity as _) };
            }
            self.capacity = capacity;
        }
    }

    impl Inner {
        const SHIFT: usize = 8;
        const CHUNK: usize = 1 << Self::SHIFT;

        #[inline]
        const fn decompose_index(index: u32) -> (u32, u8) {
            (index >> Self::SHIFT, index as u8)
        }

        #[inline]
        const fn decompose_count(count: u64) -> (u16, u16, u32) {
            ((count >> 48) as u16, (count >> 32) as u16, count as u32)
        }

        #[inline]
        const fn recompose_count(begun: u16, ended: u16, count: u32) -> u64 {
            ((begun as u64) << 48) | ((ended as u64) << 32) | (count as u64)
        }

        pub fn new() -> Self {
            Self {
                free: RwLock::new((Vec::new(), 0.into())),
                slots: (RwLock::new(0), Vec::new().into()),
                tables: Vec::new(),
            }
        }

        pub fn slot(&self, key: Key) -> Option<&Slot> {
            let count_read = self.slots.0.read();
            let (chunk_index, slot_index) = Self::decompose_index(key.index);
            // SAFETY: `chunks` can be read since the `count_read` lock is held.
            let chunks = unsafe { &**self.slots.1.get() };
            let chunk = &**chunks.get(chunk_index as usize)?;
            // SAFETY: As soon as the `chunk` is dereferenced, the `count_read` lock is no longer needed.
            drop(count_read);
            let slot = chunk.get(slot_index as usize)?;
            if slot.generation() == key.generation {
                // SAFETY: A shared reference to a slot can be returned safely without being tied to the lifetime of the read guard
                // because its address is stable and no mutable reference to it is ever given out.
                // The stability of the address is guaranteed by the fact that the `chunks` vector never drops its items other than
                // when `self` is dropped.
                Some(slot)
            } else {
                None
            }
        }

        pub unsafe fn slot_unchecked(&self, key: Key) -> &Slot {
            // See `slot` for safety.
            let count_read = self.slots.0.read();
            let (chunk_index, slot_index) = Self::decompose_index(key.index);
            let chunks = &**self.slots.1.get();
            let chunk = &**chunks.get_unchecked(chunk_index as usize);
            drop(count_read);
            chunk.get_unchecked(slot_index as usize)
        }

        pub fn create(
            &self,
            table_index: u32,
            mut initialize: impl FnMut(usize, usize, &Table),
            keys: &mut [Key],
        ) -> Option<()> {
            let table = self.tables.get(table_index as usize)?;
            // Create in batches to give a chance to other threads to make progress.
            for keys in keys.chunks_mut(Self::CHUNK) {
                let key_count = keys.len() as u16;
                // Hold this lock until the operation is fully complete such that no move operation are interleaved.
                let table_read = table.upgradable_read();
                let (store_index, store_count, table_read) =
                    Self::create_reserve(key_count, table_read);
                let mut done = 0;
                let free_read = self.free.read();
                let tail = free_read.1.fetch_sub(key_count as _, Relaxed);
                if tail > 0 {
                    let tail = tail as usize;
                    let count = tail.min(key_count as _);
                    let head = tail - count;
                    keys.copy_from_slice(&free_read.0[head..tail]);
                    drop(free_read);

                    let head = done;
                    done += count;
                    let tail = done;
                    let keys = &keys[head..tail];
                    unsafe {
                        table_read
                            .keys
                            .write_unlocked(store_index as usize..store_count, store_count)
                            .copy_from_slice(keys);
                    }
                    initialize(store_index as _, count, &table_read);
                } else {
                    drop(free_read);
                }

                if done < key_count as _ {
                    // Since all indices use `u32` for compactness, this index must remain under `u32::MAX`.
                    // Note that 'u32::MAX' is used as a sentinel so it must be an invalid entity index.
                    let keys = &mut keys[done..];
                    let index = self
                        .slot_reserve(keys.len() as u32)
                        .expect("Expected slot count to be `< u32::MAX`.");
                    for (i, key) in keys.iter_mut().enumerate() {
                        *key = Key::new(index + i as u32);
                    }

                    let head = store_index as usize + done;
                    unsafe {
                        table_read
                            .keys
                            .write_unlocked(head..store_count, store_count)
                            .copy_from_slice(keys);
                    }
                    initialize(head, store_count, &table_read);
                }

                // Initialize the slot only after the table row has been fully initialized.
                for &key in keys.iter() {
                    let slot = unsafe { self.slot_unchecked(key) };
                    slot.initialize(key.generation, table_index, store_index);
                }

                Self::create_resolve(key_count, table_read);
            }
            Some(())
        }

        /// Can be used to add or remove data associated with a key.
        pub fn modify(
            &self,
            key: Key,
            target_index: u32,
            mut initialize: impl FnMut(usize, usize, &Store),
        ) -> Option<()> {
            let target_table = self.tables.get(target_index as usize)?;
            loop {
                let slot = self.slot(key)?;
                let source_indices = slot.indices();
                if source_indices.0 == target_index {
                    // No move is needed.
                    break Some(());
                }
                let source_table = match self.tables.get(source_indices.0 as usize) {
                    Some(table) => table,
                    None => return None,
                };

                // Note that 2 very synchronized threads with their `source_table` and `target_table` swapped may
                // defeat this scheme for taking 2 write locks without dead locking. It is assumed that it doesn't
                // really happen in practice.
                let source_write = source_table.write();
                let (source_write, target_read) = match target_table.try_upgradable_read() {
                    Some(target_read) => (source_write, target_read),
                    None => {
                        drop(source_write);
                        let target_read = target_table.upgradable_read();
                        match source_table.try_write() {
                            Some(source_write) => (source_write, target_read),
                            None => continue,
                        }
                    }
                };
                if source_indices != slot.indices() {
                    continue;
                }

                let (last_index, source_write) = Self::destroy_reserve(source_write);
                let (store_index, store_count, target_read) = Self::create_reserve(1, target_read);
                let mut store_indices = (0, 0);

                fn drop_or_squash(source: u32, target: u32, store: &Store) {
                    if source == target {
                        unsafe { store.drop(target as _, 1) };
                    } else {
                        unsafe { store.squash(source as _, target as _, 1) };
                    }
                }

                loop {
                    match (
                        source_write.stores.get(store_indices.0),
                        target_read.stores.get(store_indices.1),
                    ) {
                        (Some(source_store), Some(target_store)) => {
                            let source_identifier = source_store.meta.identifier;
                            let target_identifier = target_store.meta.identifier;
                            if source_identifier == target_identifier {
                                store_indices.0 += 1;
                                store_indices.1 += 1;
                                unsafe {
                                    Store::copy(
                                        (source_store, source_indices.1 as _),
                                        (target_store, store_index as _),
                                        1,
                                    );
                                };
                                drop_or_squash(last_index, source_indices.1, source_store);
                            } else if source_identifier < target_identifier {
                                drop_or_squash(last_index, source_indices.1, source_store);
                                store_indices.0 += 1;
                            } else {
                                store_indices.1 += 1;
                                initialize(store_index as _, store_count, target_store);
                            }
                        }
                        (Some(source_store), None) => {
                            store_indices.0 += 1;
                            drop_or_squash(last_index, source_indices.1, source_store);
                        }
                        (None, Some(target_store)) => {
                            store_indices.1 += 1;
                            initialize(store_index as _, store_count, target_store);
                        }
                        (None, None) => break,
                    }
                }

                if last_index == source_indices.1 {
                    unsafe {
                        *target_read.keys.write_unlocked_at(store_index as _) = key;
                        self.slot_unchecked(key).update(target_index, store_index);
                    }
                } else {
                    unsafe {
                        let mut keys = source_write.keys.write_all::<Key>(last_index as usize + 1);
                        let last_key = *keys.get_unchecked(last_index as usize);
                        let source_key = keys.get_unchecked_mut(source_indices.1 as usize);
                        let source_key = replace(source_key, last_key);
                        *target_read.keys.write_unlocked_at(store_index as _) = source_key;
                        self.slot_unchecked(source_key)
                            .update(target_index, store_index);
                        self.slot_unchecked(last_key)
                            .update(source_indices.0, source_indices.1);
                    }
                }

                Self::create_resolve(1, target_read);
                drop(source_write);
                break Some(());
            }
        }

        pub fn destroy(&self, key: Key) -> Option<()> {
            let slot = self.slot(key)?;
            let (table_index, store_index) = slot.release(key.generation)?;
            let table = unsafe { self.tables.get_unchecked(table_index as usize) };
            let mut table_write = table.write();
            let last_index = {
                let table_count = table_write.count.get_mut();
                let (begun, ended, mut count) = Self::decompose_count(*table_count);
                // Sanity checks. If this is not the case, there is a bug in the locking logic.
                debug_assert_eq!(begun, 0u16);
                debug_assert_eq!(ended, 0u16);
                count -= 1;
                *table_count = Self::recompose_count(0, 0, count);
                count
            };

            if store_index == last_index {
                for store in table_write.stores.iter() {
                    unsafe { store.drop(store_index as _, 1) };
                }
            } else {
                for store in table_write.stores.iter() {
                    unsafe { store.squash(last_index as _, store_index as _, 1) };
                }
                unsafe {
                    let mut keys = table_write.keys.write_all::<Key>(last_index as usize + 1);
                    let last_key = *keys.get_unchecked(last_index as usize);
                    *keys.get_unchecked_mut(store_index as usize) = last_key;
                    self.slot_unchecked(last_key)
                        .update(table_index, store_index);
                }
            }

            drop(table_write);
            self.destroy_resolve(key);
            Some(())
        }

        fn slot_reserve(&self, count: u32) -> Option<u32> {
            let mut count_write = self.slots.0.write();
            let index = *count_write;
            let count = index.saturating_add(count);
            if count == u32::MAX {
                return None;
            }

            // SAFETY: `chunks` can be safely written to since the `count_write` lock is held.
            let chunks = unsafe { &mut *self.slots.1.get() };
            while count as usize > chunks.len() * Self::CHUNK {
                chunks.push(Box::new([(); Self::CHUNK].map(|_| Slot::default())));
            }
            *count_write = count;
            drop(count_write);
            Some(index)
        }

        fn create_reserve(
            reserve: u16,
            table_read: RwLockUpgradableReadGuard<Table>,
        ) -> (u32, usize, RwLockReadGuard<Table>) {
            let (begun, count) = {
                let add = Self::recompose_count(reserve, 0, 0);
                let count = table_read.count.fetch_add(add, AcqRel);
                let (begun, ended, count) = Self::decompose_count(count);
                debug_assert!(begun >= ended);
                (begun, count)
            };
            let store_index = count + begun as u32;
            let store_count = store_index as usize + reserve as usize;

            // There can not be more than `u32::MAX` keys at a given time.
            assert!(store_count < u32::MAX as _);
            let table_read = if store_count > table_read.capacity as _ {
                let mut table_write = RwLockUpgradableReadGuard::upgrade(table_read);
                table_write.grow(store_count as _);
                RwLockWriteGuard::downgrade(table_write)
            } else if begun >= u16::MAX >> 1 {
                // A huge stream of concurrent `create` operations has been detected; force the resolution of that table's `count`
                // before `begun` or `ended` overflows. This should essentially never happen.
                let mut table_write = RwLockUpgradableReadGuard::upgrade(table_read);
                let table_count = table_write.count.get_mut();
                let (begun, ended, count) = Self::decompose_count(*table_count);
                debug_assert_eq!(begun + reserve, ended);
                *table_count = Self::recompose_count(reserve, 0, count + ended as u32);
                RwLockWriteGuard::downgrade(table_write)
            } else {
                RwLockUpgradableReadGuard::downgrade(table_read)
            };
            (store_index, store_count, table_read)
        }

        fn create_resolve(reserve: u16, table_read: RwLockReadGuard<Table>) {
            table_read
                .count
                .fetch_update(AcqRel, Acquire, |count| {
                    let (begun, ended, count) = Self::decompose_count(count);
                    if begun == ended + reserve {
                        Some(Self::recompose_count(0, 0, count + begun as u32))
                    } else if begun > ended {
                        Some(Self::recompose_count(begun, ended + reserve, count))
                    } else {
                        // If this happens, this is a bug. The `expect` below will panic.
                        None
                    }
                })
                .expect("Expected the updating the count to succeed.");
        }

        fn destroy_reserve(
            mut table_write: RwLockWriteGuard<Table>,
        ) -> (u32, RwLockWriteGuard<Table>) {
            let index = {
                let table_count = table_write.count.get_mut();
                let (begun, ended, mut count) = Self::decompose_count(*table_count);
                // Sanity checks. If this is not the case, there is a bug in the locking logic.
                debug_assert_eq!(begun, 0u16);
                debug_assert_eq!(ended, 0u16);
                count -= 1;
                *table_count = Self::recompose_count(0, 0, count);
                count
            };
            (index, table_write)
        }

        fn destroy_resolve(&self, mut key: Key) {
            key.generation = key.generation.saturating_add(1);
            if key.generation < u32::MAX {
                let mut free_write = self.free.write();
                let free_count = *free_write.1.get_mut();
                free_write.0.truncate(free_count.max(0) as _);
                free_write.0.push(key);
                *free_write.1.get_mut() = free_write.0.len() as _;
                drop(free_write);
            }
        }
    }

    impl Database {
        pub fn new() -> Self {
            Self(Arc::new(Inner::new()))
        }

        // pub fn keys(&self) -> impl IntoIterator<Item = Key> + '_ {
        //     self.0.tables.iter().flat_map(|table| {
        //         let table_read = table.read();
        //         let keys_read = unsafe { table_read.keys.read(.., table_read.count() as _) };
        //         keys_read.iter().copied()
        //     })
        // }

        pub fn query<I: Item>(&self) -> Result<Query<I>, Error> {
            // TODO: Fail when an invalid query is detected (ex: `Query<(&mut Position, &mut Position)>`).
            todo!()
        }

        pub fn query_with<I: Item, F: Filter>(&self, filter: F) -> Result<Query<I, F>, Error> {
            // TODO: Fail when an invalid query is detected (ex: `Query<(&mut Position, &mut Position)>`).
            todo!()
        }

        pub fn create<T: Template>(&mut self) -> Result<Create<T>, Error> {
            // TODO: Fail when there are duplicate `Datum`?
            todo!()
        }

        pub fn destroy(&mut self) -> Destroy {
            todo!()
        }
    }

    impl<C: Datum> Item for &C {
        type State = Read<C>;

        fn initialize(table: &Table) -> Option<Self::State> {
            Some(Read(*table.indices.get(&TypeId::of::<C>())?, PhantomData))
        }
    }

    impl<'a, C: Datum> At<'a> for Read<C> {
        type State = MappedRwLockReadGuard<'a, [C]>;
        type Chunk = &'a [C];
        type Item = &'a C;

        #[inline]
        fn get(&self, table: &Table) -> Option<Self::State> {
            todo!()
            // unsafe { from_raw_parts(state.cast::<C>().as_ptr(), *count) }
            // Some((stores[self.0].data.try_read()?, count))
        }

        #[inline]
        unsafe fn chunk(state: &mut Self::State) -> Self::Chunk {
            todo!()
            // state.as_ref()
        }

        #[inline]
        unsafe fn item(state: &mut Self::State, index: usize) -> Self::Item {
            todo!()
            // unsafe { state.get_unchecked(index) }
        }
    }

    // impl<C: Component> Item for &mut C {
    //     type State = ();
    // }
    // impl<I1: Item, I2: Item> Item for (I1, I2) {
    //     type State = (I1::State, I2::State);
    // }

    impl Item for Key {
        type State = Self;

        fn initialize(table: &Table) -> Option<Self::State> {
            todo!()
        }
    }

    impl<'a> At<'a> for Key {
        type State = MappedRwLockReadGuard<'a, [Key]>;
        type Chunk = &'a [Key];
        type Item = Key;

        fn get(&self, table: &Table) -> Option<Self::State> {
            todo!()
        }

        unsafe fn chunk(state: &mut Self::State) -> Self::Chunk {
            todo!()
        }

        unsafe fn item(state: &mut Self::State, index: usize) -> Self::Item {
            todo!()
        }
    }

    impl<'a, T> Deref for Guard<'a, T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> DerefMut for Guard<'a, T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<I: Item, F: Filter> Query<I, F> {
        // pub fn item(&mut self, key: Key) -> Option<Guard<<I::State as At>::Item>> {
        //     self.with(key, |item, table| Guard(item, table))
        // }

        pub fn item_with<T>(
            &mut self,
            key: Key,
            with: impl FnOnce(<I::State as At>::Item) -> T,
        ) -> Option<T> {
            self.with(key, |item, _| with(item))
        }

        pub fn items(&mut self) -> impl Iterator<Item = <I::State as At>::Item> {
            self.iterate().flat_map(|(mut state, table)| {
                (0..table.count()).map(move |i| unsafe { I::State::item(&mut state, i as usize) })
            })
        }

        pub fn items_with(&mut self, mut each: impl FnMut(<I::State as At>::Item)) {
            self.each(|mut state, table| {
                for i in 0..table.count() {
                    each(unsafe { I::State::item(&mut state, i as usize) });
                }
            })
        }

        pub fn chunks(&mut self) -> impl Iterator<Item = <I::State as At>::Chunk> {
            self.iterate()
                .map(|(mut state, _)| unsafe { I::State::chunk(&mut state) })
        }

        pub fn chunks_with(&mut self, mut each: impl FnMut(<I::State as At>::Chunk)) {
            self.each(|mut state, _| each(unsafe { I::State::chunk(&mut state) }));
        }

        /// Ensure that all tables have been filtered or initialized.
        fn update(&mut self) {
            while let Some(table) = self.inner.tables.get(self.index) {
                let table_read = table.read();
                if self.filter.filter(&table_read) {
                    if let Some(state) = I::initialize(&table_read) {
                        drop(table_read);
                        self.indices.insert(self.index, self.states.len());
                        self.states.push((self.index, state));
                    } else {
                        drop(table_read);
                    }
                } else {
                    drop(table_read);
                }
                self.index += 1;
            }
        }

        fn with<T>(
            &mut self,
            key: Key,
            with: impl FnOnce(<I::State as At>::Item, RwLockReadGuard<Table>) -> T,
        ) -> Option<T> {
            Some(loop {
                self.update();
                let Self {
                    inner,
                    indices,
                    states,
                    ..
                } = &*self;
                let slot = self.inner.slot(key)?;
                let (table_index, store_index) = slot.indices();
                let &state_index = match indices.get(&(table_index as usize)) {
                    Some(index) => index,
                    None => return None,
                };
                let (_, item_state) = unsafe { states.get_unchecked(state_index) };
                let table = unsafe { inner.tables.get_unchecked(table_index as usize) };
                let table_read = table.read();
                if slot.indices() != (table_index, store_index) {
                    continue;
                }

                match I::State::get(item_state, &table_read) {
                    Some(mut at_state) => {
                        let item = unsafe { I::State::item(&mut at_state, store_index as _) };
                        break with(item, table_read);
                    }
                    None => {
                        drop(table_read);
                        // It is allowed that there be interleaving of other thread operations here as long as the
                        // `slot.indices` are checked each time a table lock is acquired.
                        let table_write = table.write();
                        if slot.indices() != (table_index, store_index) {
                            continue;
                        }

                        match I::State::get(item_state, &table_write) {
                            Some(mut at_state) => {
                                let table_read = RwLockWriteGuard::downgrade(table_write);
                                let item =
                                    unsafe { I::State::item(&mut at_state, store_index as _) };
                                break with(item, table_read);
                            }
                            None => unreachable!(),
                        }
                    }
                }
            })
        }

        fn iterate(
            &mut self,
        ) -> impl Iterator<Item = (<I::State as At>::State, RwLockReadGuard<Table>)> {
            from_generator(|| {
                self.update();

                // Try to execute the query using only read locks on tables. This should succeed unless there is contention over
                // the store locks which would cause the `I::State::get` call to fail.
                for (state_index, (table_index, item_state)) in self.states.iter().enumerate() {
                    if let Some(table) = self.inner.tables.get(*table_index) {
                        let table_read = table.read();
                        match I::State::get(item_state, &table_read) {
                            Some(at_state) => yield (at_state, table_read),
                            None => {
                                drop(table_read);
                                self.queue.push_back(state_index);
                            }
                        }
                    }
                }

                // Try again to execute the tables that previously failed to take their store locks by still using only read
                // locks on tables hoping that there is no more contention.
                let mut count = self.queue.len();
                while let Some(state_index) = self.queue.pop_front() {
                    if let Some((table_index, item_state)) = self.states.get(state_index) {
                        if let Some(table) = self.inner.tables.get(*table_index) {
                            let table_read = table.read();
                            match I::State::get(item_state, &table_read) {
                                Some(at_state) => {
                                    count = self.queue.len();
                                    yield (at_state, table_read);
                                }
                                None if count == 0 => {
                                    drop(table_read);
                                    // Since no table can make progress, escalate to a write lock.
                                    let table_write = table.write();
                                    match I::State::get(item_state, &table_write) {
                                        Some(at_state) => {
                                            count = self.queue.len();
                                            let table_read =
                                                RwLockWriteGuard::downgrade(table_write);
                                            yield (at_state, table_read);
                                        }
                                        None => unreachable!(),
                                    }
                                }
                                None => {
                                    drop(table_read);
                                    self.queue.push_back(state_index);
                                    count -= 1;
                                }
                            }
                        }
                    }
                }
            })
        }

        fn each(&mut self, mut each: impl FnMut(<I::State as At>::State, &Table)) {
            self.update();

            // Try to execute the query using only read locks on tables. This should succeed unless there is contention over
            // the store locks which would cause the `I::State::get` call to fail.
            for (state_index, (table_index, item_state)) in self.states.iter().enumerate() {
                if let Some(table) = self.inner.tables.get(*table_index) {
                    let table_read = table.read();
                    match I::State::get(item_state, &table_read) {
                        Some(at_state) => {
                            each(at_state, &table_read);
                            drop(table_read);
                        }
                        None => {
                            drop(table_read);
                            self.queue.push_back(state_index);
                        }
                    }
                }
            }

            // Try again to execute the tables that previously failed to take their store locks by still using only read
            // locks on tables hoping that there is no more contention.
            let mut count = self.queue.len();
            while let Some(state_index) = self.queue.pop_front() {
                if let Some((table_index, item_state)) = self.states.get(state_index) {
                    if let Some(table) = self.inner.tables.get(*table_index) {
                        let table_read = table.read();
                        match I::State::get(item_state, &table_read) {
                            Some(at_state) => {
                                each(at_state, &table_read);
                                drop(table);
                                count = self.queue.len();
                            }
                            None if count == 0 => {
                                drop(table_read);
                                // Since no table can make progress, escalate to a write lock.
                                let table_write = table.write();
                                match I::State::get(item_state, &table_write) {
                                    Some(at_state) => {
                                        let table_read = RwLockWriteGuard::downgrade(table_write);
                                        each(at_state, &table_read);
                                        drop(table_read);
                                        count = self.queue.len();
                                    }
                                    None => unreachable!(),
                                }
                            }
                            None => {
                                drop(table_read);
                                self.queue.push_back(state_index);
                                count -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    unsafe impl<D: Datum> Template for D {
        unsafe fn apply(self, store: usize, table: &Table) {
            todo!()
        }
    }

    unsafe impl Template for () {
        unsafe fn apply(self, store: usize, table: &Table) {
            todo!()
        }
    }

    impl<T: Template> Create<T> {
        pub fn one(&mut self, template: T) -> Key {
            todo!()
        }

        pub fn all<I: IntoIterator<Item = T>>(&mut self, templates: I) -> &[Key] {
            todo!()
        }

        pub fn clones(&mut self, count: usize, template: T) -> &[Key]
        where
            T: Clone,
        {
            todo!()
        }

        pub fn defaults(&mut self, count: usize) -> &[Key]
        where
            T: Default,
        {
            todo!()
        }
    }

    impl Destroy {
        pub fn one(&mut self, key: Key) -> bool {
            todo!()
        }

        /// Destroys all provided `keys` and returns the a count of the keys that were successfully destroyed.
        pub fn all<I: IntoIterator<Item = Key>>(&mut self, keys: I) -> usize {
            todo!()
        }
    }

    impl Keys {
        pub fn has(&self, key: Key) -> bool {
            todo!()
        }
    }

    impl IntoIterator for &Keys {
        type Item = Key;
        type IntoIter = impl Iterator<Item = Self::Item>;

        fn into_iter(self) -> Self::IntoIter {
            [].into_iter()
        }
    }

    fn main() -> Result<(), Error> {
        #[derive(Default, Clone)]
        struct Position;
        struct Velocity;
        impl Datum for Position {}
        impl Datum for Velocity {}

        let mut database = Database::new();
        fn filter(table: &Table) -> bool {
            true
        }
        let mut query1 = database.query_with::<&Position, _>(filter)?;
        let mut query2 = database.query::<Key>()?;
        let mut create = database.create()?;
        let mut destroy = database.destroy();

        for _item in query1.items() {}
        for _chunk in query1.chunks() {}
        query1.items_with(|_item| {});
        query1.chunks_with(|_chunk| {});
        query1.item_with(Key::NULL, |_item| {});
        create.one(Position);
        create.all([Position]);
        create.clones(100, Position);
        create.defaults(100);
        destroy.one(Key::NULL);
        destroy.all([Key::NULL]);
        destroy.all(query2.items());
        Ok(())
    }
}

pub mod boba {
    use parking_lot::Mutex;
    use std::{
        cell::UnsafeCell,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    pub struct BobaSlot<T> {
        items: UnsafeCell<Vec<T>>,
        used: AtomicBool,
    }

    pub struct BobaVec<T, const N: usize = 8> {
        slots: [BobaSlot<T>; N],
        over: Mutex<Vec<T>>,
        count: AtomicUsize,
    }

    impl<T> BobaSlot<T> {
        pub fn new() -> Self {
            Self {
                items: Vec::new().into(),
                used: false.into(),
            }
        }

        pub fn push(&self, value: T) -> Option<T> {
            if self.used.swap(true, Ordering::AcqRel) {
                // Slot is in use.
                Some(value)
            } else {
                // Slot is owned by this thread so it is safe to mutate the vector.
                unsafe { &mut *self.items.get() }.push(value);
                self.used.store(false, Ordering::Release);
                None
            }
        }

        pub fn pop(&self) -> Option<T> {
            if self.used.swap(true, Ordering::AcqRel) {
                // Slot is in use, can not pop.
                None
            } else {
                let value = unsafe { &mut *self.items.get() }.pop();
                self.used.store(false, Ordering::Release);
                value
            }
        }
    }

    impl<T, const N: usize> BobaVec<T, N> {
        #[inline]
        pub fn new() -> Self {
            Self {
                slots: [(); N].map(|_| BobaSlot::new()),
                over: Mutex::new(Vec::new()),
                count: 0.into(),
            }
        }

        #[inline]
        pub fn len(&self) -> usize {
            self.count.load(Ordering::Acquire)
        }

        pub fn resolve(&mut self) {
            let count = *self.count.get_mut();
            let over = self.over.get_mut();
            let mut offset = 1;
            for value in over.drain(..) {
                let slot = &mut self.slots[(count + offset) % N];
                slot.items.get_mut().push(value);
                offset += 1;
            }
        }

        pub fn push(&self, value: T) {
            let count = self.count.fetch_add(1, Ordering::Relaxed) % N;
            if let Some(value) = self.slots[count % N].push(value) {
                self.over.lock().push(value);
            }
        }

        pub fn pop(&self) -> Option<T> {
            let count = self
                .count
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    if count == 0 {
                        None
                    } else {
                        Some(count - 1)
                    }
                })
                .ok()?;

            loop {
                // Fast path.
                if let Some(value) = self.slots[count % N].pop() {
                    return Some(value);
                } else if let Some(value) = self.over.lock().pop() {
                    // Slot is in use, try the `Mutex`.
                    return Some(value);
                }

                // Slow path.
                // Try to find an item in the non-empty slots.
                for i in 1..N {
                    if let Some(value) = self.slots[(count + i) % N].pop() {
                        return Some(value);
                    }
                }
            }
        }
    }
}

pub mod pouahl {
    use super::*;
    use crate::store::Store;
    use std::{
        any::TypeId,
        collections::HashSet,
        sync::{
            atomic::{AtomicI64, AtomicU32},
            Arc,
        },
    };

    #[derive(Clone)]
    pub struct Entity(Arc<Datum>);

    #[derive(Clone)]
    pub struct Datum {
        index: u32,
        generation: u32,
        segment: Arc<Segment>,
        store: u32,
    }

    pub struct Family {
        parent: u32,
        first_child: u32,
        next_sibling: u32,
        // children: u32,
        // previous_sibling: u32,
        // last_child: u32,
    }

    pub struct Segment {
        identifier: usize,
        count: usize,
        capacity: usize,
        reserved: AtomicU32,
        disable_store: Arc<Store>,
        family_store: Arc<Store>,
        component_stores: Box<[Arc<Store>]>,
        component_types: HashSet<TypeId>,
    }

    pub struct Segments {
        segments: Vec<Segment>,
        data: (Vec<Datum>, AtomicU32),
        free: (Vec<u32>, AtomicI64),
    }

    /*
    Disabling components:
    - Add a `dynamic: usize` count to segments.
    - The first `0..dynamic` slots in a segment must do dynamic validation with the `disable_store` of the entities.
    - The next `dynamic..count` slots can be iterated unconditionally.
    - This allows any entity to become dynamic.
    - May require swaping entities on `Enable/Disable` so it depends on (Write<Entities>, Write<Segment>).

    OR

    - Add a `dynamic: bool` (or a flag) to segments.
    - Some segments can be created dynamic.
    - They are always iterated with dynamic validation.
    - This requires to declare an entity as dynamic at creation-time.
        - `Enable/Disable<C>` could require that `C` implements the `Disable` unsafe tag trait.
        - The `Component` trait would then have a `fn dynamic() -> bool` function that would be required to agree with
        its `Disable` implementation.
        - Then, the dynamic nature of the component would be stored in its `Meta`.
    - `Query` (or `&C`?) will depend on `Read(disable_store)`.
    - `Enable/Disable::resolve` will only depend on `Write(disable_store)`.
    */

    /*
    - BAD: Entity creation could be heavily biased to always increment the generations of the last chunk of the segment.
    - BAD: How will entities be destroyed if they can't be moved? `Destroy` needs to swap entities.

    - Creating an entity: best case.
        let entity_index = segments.free.1.fetch_sub(1, Ordering::Relaxed); // entity_index >= 0; // (since it is the best case)
        let store_index = segment.reserved.fetch_add(1, Ordering::Relaxed); // store_index < capacity; // (since it is the best case)
        let generation = generation_store.get(store_index).increment();
        // Initialize family, if required.
        // Initialize components, if any.
        Entity(generation, entity_index)

    - Creating an entity: worst case.
        let entity_index = segments.free.1.fetch_sub(1, Ordering::Relaxed); // entity_index < 0; // (since it is the best case)
        let store_index = segment.reserved.fetch_add(1, Ordering::Relaxed); // store_index < capacity; // (since it is the best case)
        let generation = generation_store.get(store_index).increment();
        // Initialize family, if required.
        // Initialize components, if any.
        Entity(generation, entity_index)

    - Get parent.
        let child_datum = segments.indices.get(child.index())?;
        let child_segment = segments.segments.get(child_datum.segment)?;
        let child_family = child_segment.family_store.get(child_datum.store);
        let parent_datum = segments.indices.get(child_family.parent)?;
        Entity(parent_datum.generation, child_family.parent)
    */
}

pub mod poulah {
    use super::*;
    use crate::{error::Result, item::Item, segment::Segment};
    use std::marker::PhantomData;

    pub struct Get<I, K: ?Sized>(PhantomData<(I, K)>);
    pub struct At<const I: usize>;
    pub struct State<S, K>(S, PhantomData<K>);

    pub trait Key<'a, K> {
        type Value;
        fn get(self) -> Self::Value;
    }

    // impl<K, I: Item> Item for Get<I, K>
    // where
    //     for<'a> <I::State as item::At<'a>>::Ref: Key<'a, K>,
    //     for<'a> <I::State as item::At<'a>>::Mut: Key<'a, K>,
    // {
    //     type State = State<I::State, K>;

    //     fn initialize(
    //         identifier: usize,
    //         segment: &Segment,
    //         world: &mut World,
    //     ) -> Result<Self::State> {
    //         Ok(State(
    //             I::initialize(identifier, segment, world)?,
    //             PhantomData,
    //         ))
    //     }

    //     fn depend(state: &Self::State) -> Vec<depend::Dependency> {
    //         todo!()
    //     }
    // }

    impl<'a, I, K, A: item::At<'a, I>> item::At<'a, I> for State<A, K>
    where
        A::Ref: Key<'a, K>,
        A::Mut: Key<'a, K>,
    {
        type State = A::State;
        type Ref = <A::Ref as Key<'a, K>>::Value;
        type Mut = <A::Mut as Key<'a, K>>::Value;

        fn get(&'a self, segment: &segment::Segment) -> Option<Self::State> {
            A::get(&self.0, segment)
        }

        unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref {
            A::at_ref(state, index).get()
        }

        unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut {
            A::at_mut(state, index).get()
        }
    }

    pub mod position2 {
        #![allow(non_camel_case_types)]

        use super::*;

        pub type X_Ref<'a, T = f64> = Get<&'a Position2<T>, keys::X>;
        pub type X_Mut<'a, T = f64> = Get<&'a mut Position2<T>, keys::X>;
        pub type Y_Ref<'a, T = f64> = Get<&'a Position2<T>, keys::Y>;
        pub type Y_Mut<'a, T = f64> = Get<&'a mut Position2<T>, keys::Y>;
        pub type At_Ref<'a, const I: usize> = Get<&'a Position2, At<I>>;
        pub type At_Mut<'a, const I: usize> = Get<&'a mut Position2, At<I>>;

        pub mod keys {
            use super::*;

            pub struct X;
            pub struct Y;
            pub struct Z;

            impl<'a, T> Key<'a, X> for &'a Position2<T> {
                type Value = &'a T;

                #[inline]
                fn get(self) -> Self::Value {
                    &self.x
                }
            }

            impl<'a, T> Key<'a, Y> for &'a Position2<T> {
                type Value = &'a T;

                #[inline]
                fn get(self) -> Self::Value {
                    &self.y
                }
            }

            impl<'a, T> Key<'a, At<0>> for &'a Position2<T> {
                type Value = &'a T;

                #[inline]
                fn get(self) -> Self::Value {
                    &self.x
                }
            }

            impl<'a, T> Key<'a, At<1>> for &'a Position2<T> {
                type Value = &'a T;

                #[inline]
                fn get(self) -> Self::Value {
                    &self.y
                }
            }

            impl<'a, T> Key<'a, X> for &'a mut Position2<T> {
                type Value = &'a mut T;

                #[inline]
                fn get(self) -> Self::Value {
                    &mut self.x
                }
            }

            impl<'a, T> Key<'a, Y> for &'a mut Position2<T> {
                type Value = &'a mut T;

                #[inline]
                fn get(self) -> Self::Value {
                    &mut self.y
                }
            }

            impl<'a, T> Key<'a, At<0>> for &'a mut Position2<T> {
                type Value = &'a mut T;

                #[inline]
                fn get(self) -> Self::Value {
                    &mut self.x
                }
            }

            impl<'a, T> Key<'a, At<1>> for &'a mut Position2<T> {
                type Value = &'a mut T;

                #[inline]
                fn get(self) -> Self::Value {
                    &mut self.y
                }
            }
        }
    }

    pub struct Position2<T = f64> {
        pub x: T,
        pub y: T,
    }
    pub struct Position3<T = f64> {
        pub x: T,
        pub y: T,
        pub z: T,
    }

    // TODO: Prevent from mutably aliasing of the same field (ex: (position2::X_Mut, position2::X_Mut)).
    // pub fn boba(mut query: Query<(position2::X_Mut, position2::Y_Ref, position2::At_Ref<1>)>) {
    //     for (_x, _y, _at) in query.iter() {}
    //     for (x, y, at) in query.iter_mut() {
    //         *x += *y;
    //         *x += *at;
    //     }
    // }
}
