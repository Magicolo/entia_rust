use crate::system::*;
use crate::world::*;
use crate::*;
use std::any::TypeId;

pub trait Component: Send + 'static {}
impl<T: Send + 'static> Component for T {}

impl<'a, C: Component> Query<'a> for &'a C {
    type State = (&'a Store<C>, usize);

    fn initialize(segment: &'a Segment, _: &World) -> Option<Self::State> {
        Some((segment.store()?, segment.index))
    }

    #[inline]
    fn query(index: usize, (store, _): &Self::State) -> Self {
        unsafe { store.at(index) }
    }

    fn dependencies((_, segment): &Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(*segment, TypeId::of::<C>())]
    }
}

impl<'a, C: Component> Query<'a> for &'a mut C {
    type State = (&'a Store<C>, usize);

    fn initialize(segment: &'a Segment, _: &World) -> Option<Self::State> {
        Some((segment.store()?, segment.index))
    }

    #[inline]
    fn query(index: usize, (store, _): &Self::State) -> Self {
        unsafe { store.at(index) }
    }

    fn dependencies((_, segment): &Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(*segment, TypeId::of::<C>())]
    }
}
