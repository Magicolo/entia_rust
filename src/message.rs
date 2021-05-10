use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::collections::VecDeque;
use std::sync::Arc;

pub trait Message: Clone + Send + 'static {}
impl<M: Clone + Send + 'static> Message for M {}

#[derive(Clone)]
pub struct Emit<'a, M: Message>(&'a Store<Messages<M>>, &'a Segment);
pub struct EmitState<M: Message>(Arc<Store<Messages<M>>>, Arc<Segment>);
pub struct Receive<'a, M: Message>(&'a mut Messages<M>);
pub struct ReceiveState<M: Message>(Arc<Store<Messages<M>>>, Arc<Segment>, usize);
pub struct Messages<M: Message>(VecDeque<M>);

impl<M: Message> Emit<'_, M> {
    // TODO: With a concurrent queue rather than a queue, messages could be emitted from any thread.
    pub fn emit(&mut self, message: M) {
        for i in 0..self.1.count {
            unsafe { self.0.at(i) }.0.push_back(message.clone());
        }
    }
}

impl<M: Message> Inject for Emit<'_, M> {
    type State = EmitState<M>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        let meta = world.meta::<Messages<M>>();
        let segment = world.segment(&[meta], Some(4));
        Some(EmitState(segment.store()?, segment))
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(
            state.1.index,
            TypeId::of::<Messages<M>>(),
        )]
    }
}

impl<'a, M: Message> Get<'a> for EmitState<M> {
    type Item = Emit<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Emit(&self.0, &self.1)
    }
}

impl<M: Message> Receive<'_, M> {
    #[inline]
    pub fn count(&self) -> usize {
        self.0 .0.len()
    }
}

impl<M: Message> Iterator for Receive<'_, M> {
    type Item = M;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0 .0.pop_front()
    }
}

impl<M: Message> Inject for Receive<'_, M> {
    type State = ReceiveState<M>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        todo!()
    }

    fn dependencies(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1.index, TypeId::of::<Messages<M>>())]
    }
}

impl<'a, M: Message> Get<'a> for ReceiveState<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(unsafe { self.0.at(self.2) })
    }
}
