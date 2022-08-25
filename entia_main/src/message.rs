use self::keep::{IntoKeep, Keep};
use crate::{
    depend::Dependency,
    error::Result,
    identify,
    inject::{Adapt, Context, Get, Inject},
    meta::Meta,
    resource::Resource,
    resource::{Read, Write},
};
use std::{collections::VecDeque, iter::FusedIterator, marker::PhantomData};

pub trait Message: Clone + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}

struct Queue<T> {
    identifier: usize,
    keep: Keep,
    items: VecDeque<T>,
}

struct Inner<T> {
    pub queues: Vec<Queue<T>>,
}

impl<T: Send + Sync + 'static> Resource for Inner<T> {}

impl<T: Send + Sync + 'static> Default for Inner<T> {
    fn default() -> Self {
        Self { queues: Vec::new() }
    }
}

pub mod keep {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Keep {
        All,
        Last(usize),
        First(usize),
    }

    pub trait IntoKeep {
        fn keep() -> Keep;
    }

    pub struct All;
    pub struct First<const N: usize>;
    pub struct Last<const N: usize>;

    impl IntoKeep for All {
        fn keep() -> Keep {
            Keep::All
        }
    }

    impl<const N: usize> IntoKeep for First<N> {
        fn keep() -> Keep {
            Keep::First(N)
        }
    }

    impl<const N: usize> IntoKeep for Last<N> {
        fn keep() -> Keep {
            Keep::Last(N)
        }
    }
}

pub mod emit {
    use super::*;

    pub struct Emit<'a, M>(&'a mut [Queue<M>]);
    pub struct State<T>(Write<Inner<T>>);

    impl<T: Clone> Emit<'_, T> {
        pub fn all(&mut self, messages: impl IntoIterator<Item = T>) {
            fn enqueue<I: IntoIterator>(queue: &mut Queue<I::Item>, messages: I) {
                match queue.keep {
                    Keep::All => queue.items.extend(messages),
                    Keep::Last(count) => {
                        for message in messages {
                            if queue.items.len() >= count {
                                queue.items.pop_front();
                            }
                            queue.items.push_back(message);
                        }
                    }
                    Keep::First(count) => {
                        let take = count.saturating_sub(queue.items.len());
                        queue.items.extend(messages.into_iter().take(take));
                    }
                }
            }

            match self.0.split_first_mut() {
                Some((head, [])) => enqueue(head, messages),
                Some((head, rest)) => {
                    // Use 'head' as a buffer for the 'rest' queues.
                    let start = head.items.len();
                    head.items.extend(messages);

                    for queue in rest {
                        enqueue(queue, head.items.range(start..).cloned());
                    }

                    // Remove overflow from 'head'.
                    match head.keep {
                        Keep::First(count) => head.items.truncate(count),
                        Keep::Last(count) => {
                            head.items.drain(..head.items.len().saturating_sub(count));
                        }
                        _ => {}
                    }
                }
                None => {}
            }
        }

        pub fn one(&mut self, message: T) {
            fn enqueue<T>(queue: &mut Queue<T>, message: T) {
                match queue.keep {
                    Keep::All => queue.items.push_back(message),
                    Keep::Last(count) => {
                        if queue.items.len() >= count {
                            queue.items.pop_front();
                        }
                        queue.items.push_back(message);
                    }
                    Keep::First(count) => {
                        if queue.items.len() < count {
                            queue.items.push_front(message);
                        }
                    }
                }
            }

            if let Some((head, rest)) = self.0.split_first_mut() {
                for queue in rest {
                    enqueue(queue, message.clone());
                }
                enqueue(head, message);
            }
        }
    }

    unsafe impl<M: Message> Inject for Emit<'_, M> {
        type Input = ();
        type State = State<M>;

        fn initialize<A: Adapt<Self::State>>(
            _: Self::Input,
            mut context: Context<Self::State, A>,
        ) -> Result<Self::State> {
            Ok(State(Write::initialize(
                None,
                context.map(|State(state)| state),
            )?))
        }

        fn depend(State(state): &Self::State) -> Vec<Dependency> {
            Write::depend(state)
        }
    }

    impl<'a, T: Send + Sync + 'static> Get<'a> for State<T> {
        type Item = Emit<'a, T>;

        #[inline]
        unsafe fn get(&'a mut self) -> Self::Item {
            Emit(&mut self.0.queues)
        }
    }
}

pub mod receive {
    use super::*;

    pub struct Receive<'a, T, K = keep::All>(&'a mut VecDeque<T>, PhantomData<K>);
    pub struct State<T, K> {
        queue: usize,
        inner: Write<Inner<T>>,
        _marker: PhantomData<fn(K)>,
    }

    impl<T, K> Receive<'_, T, K> {
        #[inline]
        pub fn clear(&mut self) {
            self.0.clear()
        }
    }

    impl<T, K> Iterator for Receive<'_, T, K> {
        type Item = T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.0.pop_front()
        }

        #[inline]
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.0.drain(0..=n).last()
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.len();
            (len, Some(len))
        }
    }

    impl<T, K> DoubleEndedIterator for Receive<'_, T, K> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.0.pop_back()
        }

        #[inline]
        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            self.0.drain(self.0.len() - n - 1..).last()
        }
    }

    impl<T, K> ExactSizeIterator for Receive<'_, T, K> {
        #[inline]
        fn len(&self) -> usize {
            self.0.len()
        }
    }

    impl<T, K> FusedIterator for Receive<'_, T, K> {}

    unsafe impl<M: Message, K: IntoKeep + 'static> Inject for Receive<'_, M, K> {
        type Input = ();
        type State = State<M, K>;

        fn initialize<A: Adapt<Self::State>>(
            _: Self::Input,
            mut context: Context<Self::State, A>,
        ) -> Result<Self::State> {
            let mut inner = Write::initialize(None, context.map(|state| &mut state.inner))?;
            let queue = {
                let index = inner.queues.len();
                inner.queues.push(Queue {
                    identifier: identify(),
                    keep: K::keep(),
                    items: VecDeque::new(),
                });
                index
            };
            Ok(State {
                queue,
                inner,
                _marker: PhantomData,
            })
        }

        fn depend(State { inner, queue, .. }: &Self::State) -> Vec<Dependency> {
            let mut dependencies = Read::depend(&inner.read());
            dependencies.push(Dependency::write_at(inner.queues[*queue].identifier));
            dependencies
        }
    }

    impl<'a, T: Send + Sync + 'static, K> Get<'a> for State<T, K> {
        type Item = Receive<'a, T, K>;

        #[inline]
        unsafe fn get(&'a mut self) -> Self::Item {
            Receive(&mut self.inner.queues[self.queue].items, PhantomData)
        }
    }
}
