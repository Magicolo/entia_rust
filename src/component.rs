use crate::item::*;
use crate::read::*;
use crate::system::*;
use crate::world::*;
use crate::write::*;

pub trait Component: Send + 'static {}
impl<T: Send + 'static> Component for T {}

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        <Read<C> as Item>::initialize(segment)
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        <Read<C> as Item>::depend(state, world)
    }
}

impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        <Write<C> as Item>::initialize(segment)
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        <Write<C> as Item>::depend(state, world)
    }
}
