use crate::{
    depend::{Depend, Dependency},
    error::Result,
    query::item::{At, Context, Item},
    world::{store::Store, World},
};
use std::{fmt, sync::Arc};

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
    type State = *const Entity;
    type Ref = Entity;
    type Mut = Self::Ref;

    #[inline]
    fn get(&'a self, _: &'a World) -> Self::State {
        self.0.data()
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Ref {
        unsafe { *state.add(index) }
    }

    #[inline]
    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Self::at(state, index)
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<Entity>().at(self.1)]
    }
}
