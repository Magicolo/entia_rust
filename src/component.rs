use crate::{
    filter::Filter, item::Item, read::Read, segment::Segment, system::Dependency, world::World,
    write::Write,
};

pub trait Component: Send + 'static {}

impl<C: Component> Filter for C {
    fn filter(segment: &Segment, _: &World) -> bool {
        segment.static_store::<C>().is_some()
    }
}

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        <Read<C> as Item>::initialize(segment, world)
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        <Read<C> as Item>::depend(state, world)
    }
}

impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        <Write<C> as Item>::initialize(segment, world)
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        <Write<C> as Item>::depend(state, world)
    }
}
