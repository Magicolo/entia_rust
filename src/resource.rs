use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub trait Resource: Default + Send + 'static {}
pub struct ReadState<R: Resource>(pub(crate) Arc<Store<R>>, pub(crate) Arc<Segment>);
pub struct WriteState<R: Resource>(pub(crate) Arc<Store<R>>, pub(crate) Arc<Segment>);

impl<R: Resource> Inject for &R {
    type State = ReadState<R>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        initialize(R::default, world).map(|pair| ReadState(pair.0, pair.1))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.push(Dependency::Read(state.1.index, TypeId::of::<R>()));
        dependencies
    }
}

impl<'a, R: Resource> Get<'a> for ReadState<R> {
    type Item = &'a R;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}

impl<R: Resource> Inject for &mut R {
    type State = WriteState<R>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        initialize(R::default, world).map(|pair| WriteState(pair.0, pair.1))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.push(Dependency::Write(state.1.index, TypeId::of::<R>()));
        dependencies
    }
}

impl<'a, R: Resource> Get<'a> for WriteState<R> {
    type Item = &'a mut R;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}

pub(crate) fn initialize<T: Send + 'static>(
    provide: impl Fn() -> T,
    world: &mut World,
) -> Option<(Arc<Store<T>>, Arc<Segment>)> {
    let meta = world.get_or_add_meta::<T>();
    let segment = world.get_or_add_segment(&[meta], Some(1));
    let store = segment.store()?;
    if segment.count.fetch_add(1, Ordering::Relaxed) == 1 {
        *unsafe { store.at(0) } = provide();
    } else {
        segment.count.fetch_sub(1, Ordering::Relaxed);
    }
    Some((store, segment))
}
