use crate::{
    depend::{Dependency, Order},
    error::Result,
    inject::{Adapt, Context},
    item::{At, Item},
    segment::{Segment, Segments},
    store::Store,
};
use std::{
    fmt,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    slice::from_raw_parts,
    sync::Arc,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}
pub struct State {
    store: Arc<Store>,
    segment: usize,
}

impl Entity {
    pub const NULL: Self = Self {
        index: u32::MAX,
        generation: u32::MAX,
    };

    #[inline]
    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub const fn identifier(&self) -> u64 {
        self.index as u64 | (self.generation as u64 >> 32)
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Entity")
            .field(&self.index)
            .field(&self.generation)
            .finish()
    }
}

impl Default for Entity {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl Item for Entity {
    type State = State;

    fn initialize<A: Adapt<Self::State>>(
        segment: &Segment,
        _: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Ok(State {
            store: segment.entity_store().clone(),
            segment: segment.identifier(),
        })
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![
            Dependency::read::<Segments>(Order::Relax),
            Dependency::read_at(state.segment, Order::Strict),
            Dependency::read::<Entity>(Order::Strict),
            Dependency::read_at(state.store.identifier(), Order::Strict),
        ]
    }
}

impl<'a> At<'a> for State {
    type State = (*const Entity, usize);
    type Ref = Entity;
    type Mut = Self::Ref;

    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        debug_assert_eq!(self.segment, segment.identifier());
        Some((self.store.data(), segment.count()))
    }

    unsafe fn at_ref(state: &Self::State, index: usize) -> Self::Ref {
        *from_raw_parts(state.0, state.1).get_unchecked(index)
    }

    unsafe fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Self::at_ref(state, index)
    }
}

macro_rules! at {
    ($r:ty) => {
        impl<'a> At<'a, $r> for State {
            type State = (*const Entity, usize);
            type Ref = &'a [Entity];
            type Mut = Self::Ref;

            fn get(&'a self, segment: &Segment) -> Option<Self::State> {
                debug_assert_eq!(self.segment, segment.identifier());
                Some((self.store.data(), segment.count()))
            }

            unsafe fn at_ref(state: &Self::State, index: $r) -> Self::Ref {
                from_raw_parts(state.0, state.1).get_unchecked(index)
            }

            unsafe fn at_mut(state: &mut Self::State, index: $r) -> Self::Mut {
                Self::at_ref(state, index)
            }
        }
    };
    ($($r:ty,)*) => { $(at!($r);)* };
}

at!(
    RangeFull,
    Range<usize>,
    RangeInclusive<usize>,
    RangeFrom<usize>,
    RangeTo<usize>,
    RangeToInclusive<usize>,
);
