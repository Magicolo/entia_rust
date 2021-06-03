use std::sync::Arc;

use crate::inject::*;
use crate::read::*;
use crate::segment::Store;
use crate::world::*;
use crate::write::*;

pub trait Resource: Default + Send + 'static {}

impl<R: Resource> Inject for &R {
    type Input = <Read<R> as Inject>::Input;
    type State = <Read<R> as Inject>::State;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        <Read<R> as Inject>::initialize(input, world)
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Read<R> as Inject>::update(state, world);
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        <Read<R> as Inject>::resolve(state, world);
    }
}

impl<R: Resource> Inject for &mut R {
    type Input = <Write<R> as Inject>::Input;
    type State = <Write<R> as Inject>::State;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        <Write<R> as Inject>::initialize(input, world)
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Write<R> as Inject>::update(state, world);
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        <Write<R> as Inject>::resolve(state, world);
    }
}

pub(crate) fn initialize<T: Send + 'static>(
    provide: impl FnOnce() -> T,
    world: &mut World,
) -> Option<(Arc<Store>, usize)> {
    let meta = world.get_or_add_meta::<T>();
    let segment = world.get_or_add_segment_by_metas(vec![meta.clone()], Some(1));
    let store = segment.store(&meta)?;
    if segment.count == 0 {
        let index = segment.reserve(1);
        unsafe { store.set(index, provide()) };
    }
    Some((store, segment.index))
}
