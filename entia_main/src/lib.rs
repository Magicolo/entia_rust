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

- Multiple components of the same type?
    - How will chunks work?
        - Query<&Position>.chunks -> &[Position] // What if an entity has more than 1 position?
        - Query<&[Position]>.chunks -> &[&[Position]]?
    - In a table, the components should be store contiguously
        - Multiply the store size by the amount of component.
        - Also multiply the indices when accessing the components (such that there can still be `u32::MAX` rows in the store).
    - They would be queried using a slice or an array.
    - When using a slice, it will include any quantity of the component (including 1).
    - When using an array, only entities with the specific `N` will be included.
    - Single component queries should still work and produce an item with the first component.
    - `|positions: &[Position]| { }`
    - `|positions: &[Position; 8]| { }`

- Allow using enums as a way to express an `Or` query.
    - ex:
        #[derive(Component)]
        struct Frozen;
        #[derive(Component)]
        struct Burnt;
        #[derive(Component)]
        struct Poisoned;
        #[derive(Item)]
        enum Status<'a> {
            Frozen(&'a Frozen),
            // Each variant may hold a sub item.
            Ouch((&'a Burnt, &'a Poisoned)),
            // If more than one variant is valid, the earlier declared one will be used.
            // - Or produce an error?
            // - Or use the more `specific` one?
            Burnt(&'a Burnt),
        }
        |query: Query<Status>| {
            for status in query.iter() {
                match status {
                    Ouch((burnt, poisoned)) => { ... },
                    _ => {}
                }
            }
        }
- Allow using any `Item` as a `Filter`.

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
