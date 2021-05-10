use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Resource: Default + Send + 'static {}
impl<T: Default + Send + 'static> Resource for T {}

pub struct ReadState<R: Resource>(Arc<Store<R>>, usize);
pub struct WriteState<R: Resource>(Arc<Store<R>>, usize);

impl<R: Resource> Inject for &R {
    type State = ReadState<R>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        /*
        let types = [TypeId::of::<R>()];
        match world.get_segment(types) {
            Some(segment) if segment.entities.len() > 0 {
                (segment.store()?, segment.index)
            }
            None => {
                let template = Template::new().add(R::default());
                let entity = world.create_entity(template);
                let (segment, _index) = world.find_segment(entity)?;
                (segment.store()?, segment.index)
            }
        }
        */

        todo!()
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1, TypeId::of::<R>())]
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
        // let segment = world.segment(&[TypeId::of::<R>()])?
        todo!()
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1, TypeId::of::<R>())]
    }
}

impl<'a, R: Resource> Get<'a> for WriteState<R> {
    type Item = &'a mut R;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}
