use std::collections::VecDeque;

// TODO: implement react?
// - Try again to add the 'Run' trait such that there may be different implementations: Every, Depend
use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    resource::Resource,
    world::World,
    write,
    write::Write,
};

pub trait Message: Clone + Send + 'static {}

struct Queue<M: Message>(usize, VecDeque<M>);
struct Inner<M: Message> {
    pub queues: Vec<Queue<M>>,
}

impl<M: Message> Resource for Inner<M> {}
impl<M: Message> Default for Inner<M> {
    fn default() -> Self {
        Self { queues: Vec::new() }
    }
}

impl<M: Message> Queue<M> {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self(capacity, VecDeque::new())
    }

    #[inline]
    pub fn enqueue(&mut self, messages: impl Iterator<Item = M>) {
        self.1.extend(messages);
        while self.0 > 0 && self.1.len() > self.0 {
            self.dequeue();
        }
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<M> {
        self.1.pop_front()
    }
}

pub mod emit {
    use super::*;

    pub struct Emit<'a, M: Message>(&'a mut Vec<M>);
    pub struct State<M: Message>(write::State<Inner<M>>, Vec<M>);

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

        fn resolve(State(inner, messages): &mut Self::State, _: InjectContext) {
            let inner = inner.as_mut();
            for queue in inner.queues.iter_mut() {
                queue.enqueue(messages.iter().cloned());
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
            vec![Dependency::defer::<M>().at(self.0.segment())]
        }
    }
}

pub mod receive {
    use super::*;

    pub struct Receive<'a, M: Message>(&'a mut Queue<M>);
    pub struct State<M: Message>(usize, write::State<Inner<M>>);

    impl<M: Message> Iterator for Receive<'_, M> {
        type Item = M;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.0.dequeue()
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
                inner.queues.push(Queue::new(input));
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
            vec![Dependency::read::<M>().at(self.1.segment())]
        }
    }
}
