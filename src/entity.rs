use crate::system::*;
use crate::world::*;
use crate::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl<'a> Query<'a> for Entity {
    type State = &'a Store<Entity>;

    fn initialize(segment: &'a Segment, _: &World) -> Option<Self::State> {
        segment.store()
    }

    #[inline]
    fn query(index: usize, store: &Self::State) -> Self {
        unsafe { *store.at(index) }
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}
