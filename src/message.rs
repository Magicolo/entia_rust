use crate::system::*;
use crate::*;
use std::marker::PhantomData;

pub trait Message: Send + 'static {}
pub struct Emit<M: Message>(PhantomData<M>);
pub struct Receive<M: Message>(PhantomData<M>);

impl<M: Message> Inject for Emit<M> {
    type State = ();
    fn initialize(world: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn update(state: &mut Self::State, world: &mut World) -> Vec<Dependency> {
        todo!()
    }
    fn resolve(state: &Self::State, world: &mut World) {}
    fn get(state: &Self::State, world: &World) -> Self {
        todo!()
    }
}

impl<M: Message> Inject for Receive<M> {
    type State = ();
    fn initialize(world: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn update(state: &mut Self::State, world: &mut World) -> Vec<Dependency> {
        todo!()
    }
    fn resolve(state: &Self::State, world: &mut World) {}
    fn get(state: &Self::State, world: &World) -> Self {
        todo!()
    }
}
