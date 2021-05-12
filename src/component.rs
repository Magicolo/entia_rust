use crate::item::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Component: Send + 'static {}
pub struct ReadState<C: Component>(Arc<Store<C>>, usize);
pub struct WriteState<C: Component>(Arc<Store<C>>, usize);

impl<C: Component> Item for &C {
    type State = ReadState<C>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(ReadState(segment.store()?, segment.index))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.push(Dependency::Read(state.1, TypeId::of::<C>()));
        dependencies
    }
}

impl<'a, C: Component> At<'a> for ReadState<C> {
    type Item = &'a C;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        unsafe { self.0.at(index) }
    }
}

impl<C: Component> Item for &mut C {
    type State = WriteState<C>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(WriteState(segment.store()?, segment.index))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.push(Dependency::Write(state.1, TypeId::of::<C>()));
        dependencies
    }
}

impl<'a, C: Component> At<'a> for WriteState<C> {
    type Item = &'a mut C;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        unsafe { self.0.at(index) }
    }
}
