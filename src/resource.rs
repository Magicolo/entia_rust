use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Resource: Default + Send + 'static {}
impl<T: Default + Send + 'static> Resource for T {}

pub struct ReadState<R: Resource>(Arc<Store<R>>, Arc<Segment>);
pub struct WriteState<R: Resource>(Arc<Store<R>>, Arc<Segment>);

impl<R: Resource> Inject for &R {
    type State = ReadState<R>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        let metas = [world.get_or_add_meta::<R>()];
        match world.get_segment(&metas) {
            Some(segment) if segment.count > 0 => Some(ReadState(segment.store()?, segment)),
            _ => {
                let (_, segment) = world.create_entity((R::default(),));
                Some(ReadState(segment.store()?, segment))
            }
        }
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1.index, TypeId::of::<R>())]
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
        let meta = world.get_or_add_meta::<R>();
        let segment = world.get_or_add_segment(&[meta], Some(1));
        let store = segment.store()?;
        Some(WriteState(store, segment))
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1.index, TypeId::of::<R>())]
    }
}

impl<'a, R: Resource> Get<'a> for WriteState<R> {
    type Item = &'a mut R;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}
