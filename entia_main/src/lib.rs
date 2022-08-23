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
pub use entia_main_derive::{Filter, Template};

pub fn identify() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod test;

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
