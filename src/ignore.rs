use self::scope::*;
use crate::{
    depend::{self, Depend, Dependency},
    error::Result,
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::World,
};
use std::marker::PhantomData;

pub struct Ignore<I, S: Scope = All>(I, PhantomData<S>);
pub struct State<T, S: Scope>(T, PhantomData<S>);

impl<I, S: Scope> Ignore<I, S> {
    /// SAFETY: Since 'Ignore' removes the dependency checks that ensure Rust's invariants, the user must maintain them through some
    /// other means. This should not be used lightly since dependency logic can be quite tricky to get right.
    #[inline]
    pub unsafe fn get(self) -> I {
        self.0
    }
}

impl<I: Inject, S: Scope> Inject for Ignore<I, S> {
    type Input = I::Input;
    type State = State<I::State, S>;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        Ok(State(I::initialize(input, context)?, PhantomData))
    }

    fn update(State(state, _): &mut Self::State, context: inject::Context) -> Result {
        I::update(state, context)
    }

    #[inline]
    fn resolve(State(state, _): &mut Self::State, context: inject::Context) -> Result {
        I::resolve(state, context)
    }
}

impl<'a, T: Get<'a>, S: Scope> Get<'a> for State<T, S> {
    type Item = Ignore<T::Item, S>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Ignore(self.0.get(world), PhantomData)
    }
}

impl<I: Item, S: Scope> Item for Ignore<I, S> {
    type State = State<I::State, S>;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Ok(State(I::initialize(context)?, PhantomData))
    }

    fn update(State(state, _): &mut Self::State, context: item::Context) -> Result {
        I::update(state, context)
    }
}

impl<'a, T: At<'a>, S: Scope> At<'a> for State<T, S> {
    type State = T::State;
    type Item = Ignore<T::Item>;

    #[inline]
    fn get(&'a self, world: &'a World) -> Self::State {
        self.0.get(world)
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Item {
        Ignore(T::at(state, index), PhantomData)
    }
}

unsafe impl<T: Depend, S: Scope> Depend for State<T, S> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        for dependency in dependencies.iter_mut() {
            *dependency = dependency.clone().ignore(S::scope());
        }
        dependencies
    }
}

pub mod scope {
    use super::*;

    pub trait Scope {
        fn scope() -> depend::Scope;
    }

    pub struct All;
    pub struct Inner;
    pub struct Outer;

    impl Scope for All {
        fn scope() -> depend::Scope {
            depend::Scope::All
        }
    }

    impl Scope for Inner {
        fn scope() -> depend::Scope {
            depend::Scope::Inner
        }
    }

    impl Scope for Outer {
        fn scope() -> depend::Scope {
            depend::Scope::Outer
        }
    }
}
