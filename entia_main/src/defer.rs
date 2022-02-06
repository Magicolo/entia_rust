use std::{any::Any, collections::VecDeque, marker::PhantomData};

use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{Context, Get, Inject},
    local::{self, Local},
    world::World,
};

pub struct Defer<'a, R: Resolve> {
    index: usize,
    indices: &'a mut Vec<(usize, usize)>,
    queue: &'a mut VecDeque<R::Item>,
}

pub struct State<T> {
    inner: local::State<Inner>,
    index: usize,
    _marker: PhantomData<T>,
}

pub trait Resolve {
    type Item;
    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) -> Result;
}

struct Resolver {
    state: Box<dyn Any + Send + Sync>,
    resolve: fn(usize, &mut dyn Any, &mut World) -> Result,
}

#[derive(Default)]
struct Inner {
    indices: Vec<(usize, usize)>,
    resolvers: Vec<Resolver>,
}

#[allow(type_alias_bounds)]
type Pair<R: Resolve> = (R, VecDeque<R::Item>);

impl Resolver {
    #[inline]
    pub fn resolve(&mut self, count: usize, world: &mut World) -> Result {
        (self.resolve)(count, self.state.as_mut(), world)
    }

    #[inline]
    pub fn state_ref<R: Resolve + 'static>(&self) -> Option<&Pair<R>> {
        self.state.downcast_ref()
    }

    #[inline]
    pub fn state_mut<R: Resolve + 'static>(&mut self) -> Option<&mut Pair<R>> {
        self.state.downcast_mut()
    }
}

impl<R: Resolve> Defer<'_, R> {
    pub fn one(&mut self, item: R::Item) {
        self.queue.push_back(item);
        self.increment(1);
    }

    pub fn all(&mut self, items: impl IntoIterator<Item = R::Item>) {
        let count = self.queue.len();
        self.queue.extend(items);
        self.increment(self.queue.len() - count);
    }

    #[inline]
    fn increment(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        match self.indices.last_mut() {
            Some(pair) if pair.0 == self.index => pair.1 += count,
            _ => self.indices.push((self.index, count)),
        }
    }
}

impl<R: Resolve + Send + Sync + 'static> Inject for Defer<'_, R>
where
    <R as Resolve>::Item: Send + Sync + 'static,
{
    type Input = R;
    type State = State<R>;

    fn initialize(input: Self::Input, context: Context) -> Result<Self::State> {
        let mut inner = Local::<Inner>::initialize(None, context)?;
        let index = {
            let inner = inner.as_mut();
            let index = inner.resolvers.len();
            inner.resolvers.push(Resolver {
                state: Box::new((input, VecDeque::<R::Item>::new())),
                resolve: |count, state, world| {
                    let (state, queue) = state
                        .downcast_mut::<Pair<R>>()
                        .expect("Invalid resolve state.");
                    state.resolve(queue.drain(..count), world)
                },
            });
            index
        };

        Ok(State {
            inner,
            index,
            _marker: PhantomData,
        })
    }

    fn resolve(state: &mut Self::State, mut context: Context) -> Result {
        let Inner { indices, resolvers } = state.inner.as_mut();
        for (index, count) in indices.drain(..) {
            resolvers[index].resolve(count, context.world())?;
        }
        Ok(())
    }
}

impl<'a, R: Resolve + 'static> Get<'a> for State<R> {
    type Item = (Defer<'a, R>, &'a mut R);

    #[inline]
    fn get(&'a mut self, _: &'a World) -> Self::Item {
        let inner = self.inner.as_mut();
        let (state, queue) = inner.resolvers[self.index].state_mut::<R>().unwrap();
        (
            Defer {
                index: self.index,
                indices: &mut inner.indices,
                queue,
            },
            state,
        )
    }
}

unsafe impl<T> Depend for State<T> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.inner.depend(world)
    }
}

impl<R: Resolve + 'static> AsRef<R> for State<R> {
    fn as_ref(&self) -> &R {
        self.inner.as_ref().resolvers[self.index]
            .state_ref::<R>()
            .map(|(state, _)| state)
            .unwrap()
    }
}

impl<R: Resolve + 'static> AsMut<R> for State<R> {
    fn as_mut(&mut self) -> &mut R {
        self.inner.as_mut().resolvers[self.index]
            .state_mut::<R>()
            .map(|(state, _)| state)
            .unwrap()
    }
}
