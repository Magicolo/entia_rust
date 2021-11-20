use std::collections::VecDeque;

use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{Context, Get, Inject},
    world::World,
    write::Write,
};

struct Queue<T>(usize, VecDeque<T>);
struct Inner<T> {
    pub queues: Vec<Queue<T>>,
}

impl<T> Default for Inner<T> {
    fn default() -> Self {
        Self { queues: Vec::new() }
    }
}

impl<T> Queue<T> {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self(capacity, VecDeque::new())
    }

    #[inline]
    pub fn enqueue(&mut self, messages: impl Iterator<Item = T>) {
        self.1.extend(messages);
        self.truncate();
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        self.1.pop_front()
    }

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

    pub struct Emit<'a, T>(&'a mut Vec<T>);
    pub struct State<T>(Write<Inner<T>>, Vec<T>);

    impl<T> Emit<'_, T> {
        #[inline]
        pub fn emit(&mut self, message: T) {
            self.0.push(message);
        }
    }

    impl<'a, T: Clone + Send + Sync + 'static> Inject for Emit<'a, T> {
        type Input = ();
        type State = State<T>;

        fn initialize(_: Self::Input, context: Context) -> Result<Self::State> {
            Ok(State(Write::initialize(None, context)?, Vec::new()))
        }

        fn resolve(State(inner, messages): &mut Self::State, _: Context) -> Result {
            let inner = inner.as_mut();
            for queue in inner.queues.iter_mut() {
                queue.enqueue(messages.iter().cloned());
            }
            messages.clear();
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

    pub struct Receive<'a, T>(&'a mut Queue<T>);
    pub struct State<T>(usize, Write<Inner<T>>);

    impl<T> Iterator for Receive<'_, T> {
        type Item = T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.0.dequeue()
        }
    }

    impl<T: Send + Sync + 'static> Inject for Receive<'_, T> {
        type Input = usize;
        type State = State<T>;

        fn initialize(input: Self::Input, context: Context) -> Result<Self::State> {
            let mut inner = Write::<Inner<T>>::initialize(None, context)?;
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
            Receive(&mut self.1.as_mut().queues[self.0])
        }
    }

    unsafe impl<T: 'static> Depend for State<T> {
        fn depend(&self, _: &World) -> Vec<Dependency> {
            vec![Dependency::read::<T>().at(self.1.segment())]
        }
    }
}
