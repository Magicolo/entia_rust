use std::{any::TypeId, collections::VecDeque};

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    message::Message,
    resource::Resource,
    world::World,
    write::{self, Write},
};

pub struct Emit<'a, M: Message>(&'a mut Vec<M>);
pub struct State<M: Message>(write::State<Inner<M>>, Vec<M>);

pub(crate) struct Queue<M: Message>(pub usize, pub VecDeque<M>);
pub(crate) struct Inner<M: Message> {
    pub queues: Vec<Queue<M>>,
}

impl<M: Message> Resource for Inner<M> {}
impl<M: Message> Default for Inner<M> {
    fn default() -> Self {
        Self { queues: Vec::new() }
    }
}

impl<M: Message> Emit<'_, M> {
    #[inline]
    pub fn emit(&mut self, message: M) {
        self.0.push(message);
    }
}

unsafe impl<'a, M: Message> Inject for Emit<'a, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, context: InjectContext) -> Option<Self::State> {
        let inner = <Write<Inner<M>> as Inject>::initialize(None, context)?;
        Some(State(inner, Vec::new()))
    }

    fn resolve(state: &mut Self::State, _: InjectContext) {
        let messages = &mut state.1;
        for queue in state.0.as_mut().queues.iter_mut() {
            queue.1.extend(messages.iter().cloned());
            while queue.0 > 0 && queue.1.len() > queue.0 {
                queue.1.pop_front();
            }
        }
        messages.clear();
    }
}

impl<M: Message> Clone for State<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<'a, M: Message> Get<'a> for State<M> {
    type Item = Emit<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Emit(&mut self.1)
    }
}

unsafe impl<M: Message> Depend for State<M> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Defer(self.0.segment(), TypeId::of::<M>())]
    }
}
