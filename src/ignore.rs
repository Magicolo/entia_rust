use std::marker::PhantomData;

use crate::{
    depend::{self, Depend, Dependency},
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::World,
};

pub struct Ignore<I, S: Scope = All>(I, PhantomData<S>);
pub struct State<T, S: Scope>(T, PhantomData<S>);

pub struct All;
pub struct Inner;
pub struct Outer;
pub trait Scope {
    fn scope() -> depend::Scope;
}

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

    fn initialize(input: Self::Input, context: inject::Context) -> Option<Self::State> {
        Some(State(I::initialize(input, context)?, PhantomData))
    }
}

impl<'a, T: Get<'a>, S: Scope> Get<'a> for State<T, S> {
    type Item = Ignore<T::Item, S>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Ignore(self.0.get(world), PhantomData)
    }
}

impl<I: Item, S: Scope> Item for Ignore<I, S> {
    type State = State<I::State, S>;

    fn initialize(context: item::Context) -> Option<Self::State> {
        Some(State(I::initialize(context)?, PhantomData))
    }
}

impl<'a, T: At<'a>, S: Scope> At<'a> for State<T, S> {
    type Item = Ignore<T::Item>;

    #[inline]
    fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
        Ignore(self.0.at(index, world), PhantomData)
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
