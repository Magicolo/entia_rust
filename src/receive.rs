use std::{any::TypeId, collections::VecDeque};

use crate::{
    depend::{Depend, Dependency},
    emit::{Inner, Queue},
    inject::{Get, Inject, InjectContext},
    message::Message,
    world::World,
    write::{self, Write},
};

pub struct Receive<'a, M: Message>(&'a mut Queue<M>);
pub struct State<M: Message>(usize, write::State<Inner<M>>);

impl<M: Message> Iterator for Receive<'_, M> {
    type Item = M;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0 .1.pop_front()
    }
}

unsafe impl<M: Message> Inject for Receive<'_, M> {
    type Input = usize;
    type State = State<M>;

    fn initialize(input: Self::Input, context: InjectContext) -> Option<Self::State> {
        let mut inner = <Write<Inner<M>> as Inject>::initialize(None, context)?;
        let index = {
            let inner = inner.as_mut();
            let index = inner.queues.len();
            inner.queues.push(Queue(input, VecDeque::new()));
            index
        };
        Some(State(index, inner))
    }
}

impl<M: Message> Clone for State<M> {
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone())
    }
}

impl<'a, M: Message> Get<'a> for State<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(&mut self.1.as_mut().queues[self.0])
    }
}

unsafe impl<M: Message> Depend for State<M> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(self.1.segment(), TypeId::of::<M>())]
    }
}
