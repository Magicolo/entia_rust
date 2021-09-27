use crate::depend::Depend;
use crate::world::*;
use crate::{depend::Dependency, segment::*};
use crate::{filter::Filter, item::*};
use std::any::TypeId;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}
pub struct State(pub(crate) Arc<Store>, usize);

impl Entity {
    pub const ZERO: Self = Self {
        index: 0,
        generation: 0,
    };
}

impl Default for Entity {
    #[inline]
    fn default() -> Self {
        Self::ZERO
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
        vec![Dependency::Read(self.1, TypeId::of::<Entity>())]
    }
}
