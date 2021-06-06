use std::{
    any::{Any, TypeId},
    collections::VecDeque,
    marker::PhantomData,
};

use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Context, Get, Inject},
    local::{self, Local},
    world::World,
};

pub struct Defer<'a, R: Resolve> {
    inner: &'a mut Inner,
    index: usize,
    _marker: PhantomData<R>,
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

    pub fn defer<R: Resolve>(&mut self, resolve: R) -> Option<&R> {
        let (store, _) = self.state_mut()?;
        store.push_back(resolve);
        store.back()
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
    #[inline]
    pub fn defer(&mut self, resolve: R) -> Option<&R> {
        let resolve = self.inner.resolvers[self.index].defer(resolve)?;
        match self.inner.defer.last_mut() {
            Some((index, count)) if *index == self.index => *count += 1,
            _ => self.inner.defer.push((self.index, 1)),
        }
        Some(resolve)
    }
}

impl<R: Resolve> Inject for Defer<'_, R> {
    type Input = R::State;
    type State = State<R>;

    fn initialize(input: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let mut inner = <Local<Inner> as Inject>::initialize((), context, world)?;
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

    fn resolve(state: &mut Self::State, world: &mut World) {
        let inner = state.inner.as_mut();
        for (index, count) in inner.defer.drain(..) {
            inner.resolvers[index].resolve(count, world);
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
        Defer {
            inner: self.inner.as_mut(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

unsafe impl<R: Resolve> Depend for State<R> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        // TODO: Find a way to reduce dependencies.
        vec![Dependency::Defer(usize::MAX, TypeId::of::<Entity>())]
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
