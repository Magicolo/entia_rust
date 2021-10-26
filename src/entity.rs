use std::{fmt, sync::Arc};

use crate::{
    depend::{Depend, Dependency},
    query::{
        filter::Filter,
        item::{At, Item, ItemContext},
    },
    world::{segment::Segment, store::Store, World},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}
pub struct State(pub(crate) Arc<Store>, usize);

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

impl Filter for Entity {
    fn filter(segment: &Segment, world: &World) -> bool {
        if let Some(meta) = world.get_meta::<Entity>() {
            segment.has(&meta)
        } else {
            false
        }
    }
}

unsafe impl Item for Entity {
    type State = State;

    fn initialize(mut context: ItemContext) -> Option<Self::State> {
        let meta = context.world().get_meta::<Entity>()?;
        let segment = context.segment();
        let store = segment.store(&meta)?;
        Some(State(store, segment.index))
    }
}

impl<'a> At<'a> for State {
    type Item = Entity;

    #[inline]
    fn at(&self, index: usize, _: &'a World) -> Self::Item {
        *unsafe { self.0.get(index) }
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<Entity>().at(self.1)]
    }
}
