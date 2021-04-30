use crate::system::*;
use crate::world::*;
use crate::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Query for Entity {
    type State = Arc<Store<Entity>>;

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        Some((segment.entities.clone(), Vec::new()))
    }

    #[inline]
    fn get(index: usize, store: &Self::State) -> Self {
        unsafe { *store.at(index) }
    }
}
