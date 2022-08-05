use self::scope::*;
use crate::{
    depend::{self, Depend, Dependency},
    error::Result,
    inject::{Get, Inject},
    item::{At, Item},
    segment::Segment,
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

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        Ok(State(I::initialize(input, identifier, world)?, PhantomData))
    }

    fn update(State(state, _): &mut Self::State, world: &mut World) -> Result {
        I::update(state, world)
    }

    #[inline]
    fn resolve(State(state, _): &mut Self::State) -> Result {
        I::resolve(state)
    }
}

impl<'a, T: Get<'a>, S: Scope> Get<'a> for State<T, S> {
    type Item = Ignore<T::Item, S>;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        Ignore(self.0.get(), PhantomData)
    }
}

impl<I: Item, S: Scope> Item for Ignore<I, S> {
    type State = State<I::State, S>;

    fn initialize(identifier: usize, segment: &Segment, world: &mut World) -> Result<Self::State> {
        Ok(State(
            I::initialize(identifier, segment, world)?,
            PhantomData,
        ))
    }
}

impl<'a, I, A: At<'a, I>, S: Scope> At<'a, I> for State<A, S> {
    type State = A::State;
    type Ref = A::Ref;
    type Mut = A::Mut;

    #[inline]
    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        A::get(&self.0, segment)
    }

    #[inline]
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref {
        A::at_ref(state, index)
    }

    #[inline]
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut {
        A::at_mut(state, index)
    }
}

unsafe impl<T: Depend, S: Scope> Depend for State<T, S> {
    fn depend(&self) -> Vec<Dependency> {
        let mut dependencies = self.0.depend();
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
