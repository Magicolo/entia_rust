use crate::*;
use std::marker::PhantomData;

pub trait Message: Send + 'static {}
pub struct Emit<M: Message>(PhantomData<M>);
pub struct Receive<M: Message>(PhantomData<M>);

impl<M: Message> Inject<'_> for Emit<M> {
    type State = ();

    fn initialize(world: &World) -> Option<Self::State> {
        todo!()
    }

    fn inject(state: &Self::State) -> Self {
        todo!()
    }
}

impl<M: Message> Inject<'_> for Receive<M> {
    type State = ();

    fn initialize(world: &World) -> Option<Self::State> {
        todo!()
    }

    fn inject(state: &Self::State) -> Self {
        todo!()
    }
}
