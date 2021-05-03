use crate::system::*;
use crate::world::*;
use crate::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Component: Send + 'static {}

impl<C: Component> Query for &C {
    type State = Arc<Store<C>>;

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        Some((
            segment.store()?,
            vec![Dependency::Read(segment.index, TypeId::of::<C>())],
        ))
    }

    #[inline]
    fn query(index: usize, store: &Self::State) -> Self {
        unsafe { store.at(index) }
    }
}

impl<C: Component> Query for &mut C {
    type State = Arc<Store<C>>;

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        Some((
            segment.store()?,
            vec![Dependency::Write(segment.index, TypeId::of::<C>())],
        ))
    }

    #[inline]
    fn query(index: usize, store: &Self::State) -> Self {
        unsafe { store.at(index) }
    }
}
