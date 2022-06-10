use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{Context, Get, Inject},
    world::{meta::Meta, World},
    write::Write,
    Resource,
};
use std::{collections::VecDeque, iter::FusedIterator};

pub trait Message: Sized + Clone + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}

struct Queue<T>(usize, VecDeque<T>);

struct Inner<T> {
    pub queues: Vec<Queue<T>>,
}

impl<T: Send + Sync + 'static> Resource for Inner<T> {
    fn initialize(_: &Meta, _: &mut World) -> Result<Self> {
        Ok(Self { queues: Vec::new() })
    }
}

impl<T> Queue<T> {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self(capacity, VecDeque::new())
    }

    #[inline]
    pub fn enqueue(&mut self, messages: impl IntoIterator<Item = T>) {
        self.1.extend(messages);
        self.truncate();
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        self.1.pop_front()
    }

    #[inline]
    fn truncate(&mut self) {
        if self.0 > 0 {
            while self.1.len() > self.0 {
                self.dequeue();
            }
        }
    }
}

pub mod emit {
    use super::*;

    pub struct Emit<'a, M>(&'a mut Vec<M>);
    pub struct State<T>(Write<Inner<T>>, Vec<T>);

    impl<T> Emit<'_, T> {
        #[inline]
        pub fn all(&mut self, messages: impl IntoIterator<Item = T>) {
            self.0.extend(messages);
        }

        #[inline]
        pub fn one(&mut self, message: T) {
            self.0.push(message.into());
        }

        #[inline]
        pub fn clear(&mut self) {
            self.0.clear()
        }
    }

    impl<'a, M: Message> Inject for Emit<'a, M> {
        type Input = ();
        type State = State<M>;

        fn initialize(_: Self::Input, context: Context) -> Result<Self::State> {
            Ok(State(Write::initialize(None, context)?, Vec::new()))
        }

        fn resolve(State(inner, messages): &mut Self::State, _: Context) -> Result {
            let inner = inner.as_mut();
            let mut iterator = inner.queues.iter_mut();
            if let Some(first) = iterator.next() {
                for queue in iterator {
                    queue.enqueue(messages.iter().cloned());
                }
                first.enqueue(messages.drain(..));
            } else {
                messages.clear();
            }
            Ok(())
        }
    }

    impl<'a, T: 'a> Get<'a> for State<T> {
        type Item = Emit<'a, T>;

        #[inline]
        fn get(&'a mut self, _: &'a World) -> Self::Item {
            Emit(&mut self.1)
        }
    }

    unsafe impl<T: 'static> Depend for State<T> {
        fn depend(&self, _: &World) -> Vec<Dependency> {
            vec![Dependency::defer::<T>().at(self.0.segment())]
        }
    }
}

pub mod receive {
    use super::*;

    pub struct Receive<'a, T>(&'a mut VecDeque<T>);
    pub struct State<T>(usize, Write<Inner<T>>);

    impl<T> Receive<'_, T> {
        #[inline]
        pub fn len(&self) -> usize {
            self.0.len()
        }

        #[inline]
        pub fn clear(&mut self) {
            self.0.clear()
        }

        #[inline]
        pub fn first(&mut self) -> Option<T> {
            self.0.pop_front()
        }

        #[inline]
        pub fn last(&mut self) -> Option<T> {
            self.0.pop_back()
        }
    }

    impl<T> Iterator for &mut Receive<'_, T> {
        type Item = T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            Receive::first(self)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = Receive::len(self);
            (len, Some(len))
        }
    }

    impl<T> DoubleEndedIterator for &mut Receive<'_, T> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            Receive::last(self)
        }
    }

    impl<T> ExactSizeIterator for &mut Receive<'_, T> {
        #[inline]
        fn len(&self) -> usize {
            Receive::len(self)
        }
    }

    impl<T> FusedIterator for &mut Receive<'_, T> {}

    impl<M: Message> Inject for Receive<'_, M> {
        type Input = usize;
        type State = State<M>;

        fn initialize(input: Self::Input, context: Context) -> Result<Self::State> {
            let mut inner = Write::<Inner<M>>::initialize(None, context)?;
            let index = {
                let inner = inner.as_mut();
                let index = inner.queues.len();
                inner.queues.push(Queue::new(input));
                index
            };
            Ok(State(index, inner))
        }
    }

    impl<'a, T: 'static> Get<'a> for State<T> {
        type Item = Receive<'a, T>;

        #[inline]
        fn get(&'a mut self, _: &World) -> Self::Item {
            Receive(&mut self.1.as_mut().queues[self.0].1)
        }
    }

    unsafe impl<T: 'static> Depend for State<T> {
        fn depend(&self, _: &World) -> Vec<Dependency> {
            vec![Dependency::read::<T>().at(self.1.segment())]
        }
    }
}
