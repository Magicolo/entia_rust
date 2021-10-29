use std::{any::Any, collections::VecDeque, marker::PhantomData};

use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Context, Get, Inject},
    local::{self, Local},
    world::World,
};

pub struct Defer<'a, R: Resolve> {
    defer: &'a mut Vec<(usize, usize)>,
    store: &'a mut VecDeque<R>,
    state: &'a mut R::State,
    index: usize,
}

pub struct State<R: Resolve> {
    inner: local::State<Inner>,
    index: usize,
    _marker: PhantomData<R>,
}

pub trait Resolve: Send + 'static {
    type State: Send;
    fn resolve(items: impl Iterator<Item = Self>, state: &mut Self::State, world: &mut World);
}

struct Resolver {
    state: Box<dyn Any + Send>,
    resolve: fn(usize, &mut dyn Any, &mut World),
}

#[derive(Default)]
struct Inner {
    defer: Vec<(usize, usize)>,
    resolvers: Vec<Resolver>,
}

#[allow(type_alias_bounds)]
type Pair<R: Resolve> = (VecDeque<R>, R::State);

impl Resolver {
    pub fn new<R: Resolve>(state: R::State) -> Self {
        Resolver {
            state: Box::new((VecDeque::<R>::new(), state)),
            resolve: |count, state, world| {
                if let Some((store, state)) = state.downcast_mut::<Pair<R>>() {
                    R::resolve(store.drain(..count), state, world);
                }
            },
        }
    }

    #[inline]
    pub fn resolve(&mut self, count: usize, world: &mut World) {
        (self.resolve)(count, self.state.as_mut(), world);
    }

    #[inline]
    pub fn state_ref<R: Resolve>(&self) -> Option<&Pair<R>> {
        self.state.downcast_ref()
    }

    #[inline]
    pub fn state_mut<R: Resolve>(&mut self) -> Option<&mut Pair<R>> {
        self.state.downcast_mut()
    }
}

impl<R: Resolve> Defer<'_, R> {
    pub fn defer(&mut self, resolve: R) -> &R {
        self.store.push_back(resolve);
        let resolve = self.store.back().unwrap();
        match self.defer.last_mut() {
            Some((index, count)) if *index == self.index => *count += 1,
            _ => self.defer.push((self.index, 1)),
        }
        resolve
    }

    #[inline]
    pub fn state(&mut self) -> &mut R::State {
        self.state
    }
}

impl<R: Resolve> Inject for Defer<'_, R> {
    type Input = R::State;
    type State = State<R>;

    fn initialize(input: Self::Input, context: Context) -> Option<Self::State> {
        let mut inner = <Local<Inner> as Inject>::initialize((), context)?;
        let index = {
            let inner = inner.as_mut();
            let index = inner.resolvers.len();
            inner.resolvers.push(Resolver::new::<R>(input));
            index
        };

        Some(State {
            inner,
            index,
            _marker: PhantomData,
        })
    }

    fn resolve(state: &mut Self::State, mut context: Context) {
        let inner = state.inner.as_mut();
        for (index, count) in inner.defer.drain(..) {
            inner.resolvers[index].resolve(count, context.world());
        }
    }
}

impl<R: Resolve> Clone for State<R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<'a, R: Resolve> Get<'a> for State<R> {
    type Item = Defer<'a, R>;

    fn get(&'a mut self, _: &'a World) -> Self::Item {
        let inner = self.inner.as_mut();
        let (store, state) = inner.resolvers[self.index].state_mut().unwrap();
        Defer {
            defer: &mut inner.defer,
            store,
            state,
            index: self.index,
        }
    }
}

unsafe impl<R: Resolve> Depend for State<R> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        // TODO: Find a way to reduce dependencies.
        vec![Dependency::defer::<Entity>()]
    }
}

impl<R: Resolve> AsRef<R::State> for State<R> {
    fn as_ref(&self) -> &R::State {
        self.inner.as_ref().resolvers[self.index]
            .state_ref::<R>()
            .map(|(_, state)| state)
            .unwrap()
    }
}

impl<R: Resolve> AsMut<R::State> for State<R> {
    fn as_mut(&mut self) -> &mut R::State {
        self.inner.as_mut().resolvers[self.index]
            .state_mut::<R>()
            .map(|(_, state)| state)
            .unwrap()
    }
}
