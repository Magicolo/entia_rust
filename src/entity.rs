use crate::item::*;
use crate::system::*;
use crate::world::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

pub struct EntityState(Arc<Store<Entity>>);

impl Item for Entity {
    type State = EntityState;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        todo!()
        // segment.store()
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

impl At<'_> for EntityState {
    type Item = Entity;

    #[inline]
    fn at(&self, index: usize) -> Self::Item {
        unsafe { *self.0.at(index) }
    }
}
