use crate::item::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Component: Send + 'static {}
impl<T: Send + 'static> Component for T {}

pub struct ReadState<C: Component>(Arc<Store<C>>, usize);
pub struct WriteState<C: Component>(Arc<Store<C>>, usize);

impl<C: Component> Item for &C {
    type State = ReadState<C>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        todo!()
        // Some((segment.store()?, segment.index))
    }

    fn dependencies(state: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(state.1, TypeId::of::<C>())]
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
        todo!()
        // Some((segment.store()?, segment.index))
    }

    fn dependencies(state: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<C>())]
    }
}

impl<'a, C: Component> At<'a> for WriteState<C> {
    type Item = &'a mut C;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        unsafe { self.0.at(index) }
    }
}
