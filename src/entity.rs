use crate::system::*;
use crate::world::*;
use crate::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Query<'_> for Entity {
    type State = Arc<Store<Entity>>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        Some(segment.entities.clone())
    }

    #[inline]
    fn query(index: usize, store: &Self::State) -> Self {
        unsafe { *store.at(index) }
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}
