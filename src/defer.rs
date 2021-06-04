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
    fn resolve(self, state: &mut Self::State, world: &mut World);
}

struct Resolver {
    state: Box<dyn Any + Send>,
    resolve: fn(&mut dyn Any, &mut World) -> bool,
}

#[derive(Default)]
struct Inner {
    defer: Vec<usize>,
    resolvers: Vec<Resolver>,
}

#[allow(type_alias_bounds)]
type Pair<R: Resolve> = (VecDeque<R>, R::State);

impl Resolver {
    pub fn new<R: Resolve>(state: R::State) -> Self {
        Resolver {
            state: Box::new((VecDeque::<R>::new(), state)),
            resolve: |state, world| {
                if let Some((store, state)) = state.downcast_mut::<Pair<R>>() {
                    if let Some(resolve) = store.pop_front() {
                        resolve.resolve(state, world);
                        return true;
                    }
                }
                false
            },
        }
    }

    pub fn defer<R: Resolve>(&mut self, resolve: R) -> Option<&R> {
        let (store, _) = self.state_mut()?;
        store.push_back(resolve);
        store.back()
    }

    #[inline]
    pub fn resolve(&mut self, world: &mut World) -> bool {
        (self.resolve)(self.state.as_mut(), world)
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
        self.inner.defer.push(self.index);
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
        for index in inner.defer.drain(..) {
            inner.resolvers[index].resolve(world);
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
    fn depend(&self, world: &World) -> Vec<Dependency> {
        world
            .segments
            .iter()
            .map(|segment| Dependency::Defer(segment.index, TypeId::of::<Entity>()))
            .collect()
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
