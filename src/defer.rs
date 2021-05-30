use std::{
    any::{Any, TypeId},
    collections::{HashMap, VecDeque},
    marker::PhantomData,
};

use crate::{
    inject::{Get, Inject},
    resource::Resource,
    system::Dependency,
    world::World,
    write::{self, Write},
};

pub struct Defer<'a, R: Resolve> {
    inner: &'a mut Inner,
    index: usize,
    _marker: PhantomData<R>,
}

pub struct State<R: Resolve> {
    inner: write::State<Inner>,
    index: usize,
    _marker: PhantomData<R>,
}

pub trait Resolve: Send + 'static {
    type State: Send;
    fn initialize(world: &mut World) -> Option<Self::State>;
    fn resolve(self, state: &mut Self::State, world: &mut World);
}

struct Resolver {
    state: Box<dyn Any + Send>,
    resolve: fn(&mut dyn Any, &mut World),
}

#[derive(Default)]
struct Inner {
    defer: Vec<usize>,
    resolvers: Vec<Resolver>,
    indices: HashMap<TypeId, usize>,
}

impl Resource for Inner {}

impl Resolver {
    pub fn new<R: Resolve>(state: R::State) -> Self {
        Resolver {
            state: Box::new((VecDeque::<R>::new(), state)),
            resolve: |state, world| {
                let (store, state) = state.downcast_mut::<(VecDeque<R>, R::State)>().unwrap();
                store.pop_front().unwrap().resolve(state, world);
            },
        }
    }

    pub fn defer<R: Resolve>(&mut self, resolve: R) {
        let (store, _) = &mut self
            .state
            .downcast_mut::<(VecDeque<R>, R::State)>()
            .unwrap();
        store.push_back(resolve);
    }

    #[inline]
    pub fn resolve(&mut self, world: &mut World) {
        (self.resolve)(&mut self.state, world);
    }
}

impl<R: Resolve> Defer<'_, R> {
    #[inline]
    pub fn defer(&mut self, resolve: R) {
        self.inner.resolvers[self.index].defer(resolve);
        self.inner.defer.push(self.index);
    }
}

impl<R: Resolve> Inject for Defer<'_, R> {
    type Input = ();
    type State = State<R>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let mut inner = <Write<Inner> as Inject>::initialize(None, world)?;
        let key = TypeId::of::<R>();
        let index = {
            let inner = inner.as_mut();
            match inner.indices.get(&key) {
                Some(&index) => index,
                None => {
                    let state = R::initialize(world)?;
                    let index = inner.resolvers.len();
                    inner.indices.insert(key, inner.resolvers.len());
                    inner.resolvers.push(Resolver::new::<R>(state));
                    index
                }
            }
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

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<'a, R: Resolve> Get<'a> for State<R> {
    type Item = Defer<'a, R>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Defer {
            inner: self.inner.get(world),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<R: Resolve> AsRef<R::State> for State<R> {
    fn as_ref(&self) -> &R::State {
        self.inner.as_ref().resolvers[self.index]
            .state
            .downcast_ref()
            .unwrap()
    }
}

impl<R: Resolve> AsMut<R::State> for State<R> {
    fn as_mut(&mut self) -> &mut R::State {
        self.inner.as_mut().resolvers[self.index]
            .state
            .downcast_mut()
            .unwrap()
    }
}
