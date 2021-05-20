use crate::item::*;
use crate::segment::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}
pub struct State(Arc<Store<Entity>>, usize);

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

impl Item for Entity {
    type State = State;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(State(segment.static_store()?, segment.index))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1, TypeId::of::<Entity>())]
    }
}

impl At<'_> for State {
    type Item = Entity;

    #[inline]
    fn at(&self, index: usize) -> Self::Item {
        unsafe { *self.0.at(index) }
    }
}
