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
pub struct State(pub(crate) Arc<Store<Entity>>, usize);

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
    fn filter(segment: &Segment, _: &World) -> bool {
        segment.static_store::<Entity>().is_some()
    }
}

impl Item for Entity {
    type State = State;

    fn initialize(segment: &Segment, _: &World) -> Option<Self::State> {
        Some(State(segment.static_store()?, segment.index))
    }
}

impl<'a> At<'a> for State {
    type Item = Entity;

    #[inline]
    fn at(&self, index: usize) -> Self::Item {
        unsafe { *self.0.at(index) }
    }
}

impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(self.1, TypeId::of::<Entity>())]
    }
}
