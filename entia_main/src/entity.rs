use crate::{
    depend::{Depend, Dependency},
    error::Result,
    item::{At, Chunk, Context, Item},
    segment::Segment,
    store::Store,
    world::World,
};
use std::{
    fmt,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
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

impl<'a> Chunk<'a> for State {
    type Ref = &'a [Entity];
    type Mut = Self::Ref;

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        debug_assert_eq!(self.1, segment.index());
        Some(unsafe { self.0.get_all(segment.count()) })
    }

    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        self.chunk(segment)
    }
}

impl<'a> At<'a, usize> for [Entity] {
    type Ref = Entity;
    type Mut = Self::Ref;

    #[inline]
    fn at(&'a self, index: usize) -> Option<Self::Ref> {
        Some(*self.get(index)?)
    }

    #[inline]
    unsafe fn at_unchecked(&'a self, index: usize) -> Self::Ref {
        *self.get_unchecked(index)
    }

    #[inline]
    fn at_mut(&'a mut self, index: usize) -> Option<Self::Mut> {
        Self::at(self, index)
    }

    #[inline]
    unsafe fn at_unchecked_mut(&'a mut self, index: usize) -> Self::Mut {
        Self::at_unchecked(self, index)
    }
}

macro_rules! at {
    ($r:ty) => {
        impl<'a> At<'a, $r> for [Entity] {
            type Ref = &'a [Entity];
            type Mut = &'a mut [Entity];

            #[inline]
            fn at(&'a self, index: $r) -> Option<Self::Ref> {
                self.get(index)
            }

            #[inline]
            unsafe fn at_unchecked(&'a self, index: $r) -> Self::Ref {
                self.get_unchecked(index)
            }

            #[inline]
            fn at_mut(&'a mut self, index: $r) -> Option<Self::Mut> {
                self.get_mut(index)
            }

            #[inline]
            unsafe fn at_unchecked_mut(&'a mut self, index: $r) -> Self::Mut {
                self.get_unchecked_mut(index)
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
