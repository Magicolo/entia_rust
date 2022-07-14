use crate::{
    depend::{Depend, Dependency},
    error::Result,
    item::{At, Context, Item},
    segment::Segment,
    store::Store,
    world::World,
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
pub struct State(Arc<Store>, usize);

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

    fn initialize(context: Context) -> Result<Self::State> {
        let segment = context.segment();
        Ok(State(segment.entity_store(), segment.index()))
    }
}

impl<'a> At<'a> for State {
    type State = (*const Entity, usize);
    type Ref = Entity;
    type Mut = Self::Ref;

    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        debug_assert_eq!(self.1, segment.index());
        Some((self.0.data(), segment.count()))
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
                debug_assert_eq!(self.1, segment.index());
                Some((self.0.data(), segment.count()))
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

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<Entity>().segment(self.1)]
    }
}
