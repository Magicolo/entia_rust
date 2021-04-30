use crate::system::*;
use crate::world::*;
use crate::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Resource: Send + 'static {}

impl<R: Resource> Inject for &R {
    type State = (Arc<Store<R>>, usize);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let segment = world.segment(&[TypeId::of::<R>()]);
        Some((segment.store()?, segment.index))
    }

    fn update((_, segment): &mut Self::State, _: &mut World) -> Vec<Dependency> {
        vec![Dependency::Read(*segment, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State, _: &mut World) {}

    #[inline]
    fn get((store, _): &Self::State, _: &World) -> Self {
        unsafe { store.at(0) }
    }
}

impl<R: Resource> Inject for &mut R {
    type State = (Arc<Store<R>>, usize);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let segment = world.segment(&[TypeId::of::<R>()]);
        Some((segment.store()?, segment.index))
    }

    fn update((_, segment): &mut Self::State, _: &mut World) -> Vec<Dependency> {
        vec![Dependency::Write(*segment, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State, _: &mut World) {}

    #[inline]
    fn get((store, _): &Self::State, _: &World) -> Self {
        unsafe { &mut *store.at(0) }
    }
}
