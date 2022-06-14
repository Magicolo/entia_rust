use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{meta::Meta, store::Store, Component, Resource, World},
    write::Write,
};

pub struct Read<T>(Write<T>);

impl<T> Read<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.0.segment()
    }

    #[inline]
    pub fn store(&self) -> &Store {
        self.0.store()
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        self.0.meta()
    }
}

impl<R: Resource> Inject for &R {
    type Input = <Read<R> as Inject>::Input;
    type State = <Read<R> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Read<_> as Inject>::initialize(input, context)
    }
}

impl<R: Resource> Inject for Read<R> {
    type Input = <Write<R> as Inject>::Input;
    type State = Self;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Write<_> as Inject>::initialize(input, context).map(Read)
    }
}

impl<'a, T: Send + Sync + 'static> Get<'a> for Read<T> {
    type Item = <Self as At<'a>>::Mut;

    #[inline]
    fn get(&'a mut self, world: &World) -> Self::Item {
        Self::at(&mut At::get(self, world), 0)
    }
}

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Read<_> as Item>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Read<T> {
    type State = Self;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Write<_> as Item>::initialize(context).map(Read)
    }
}

impl<'a, T: Send + Sync + 'static> At<'a> for Read<T> {
    type State = *const T;
    type Ref = &'a T;
    type Mut = Self::Ref;

    #[inline]
    fn get(&'a self, world: &'a World) -> Self::State {
        self.0.get(world) as _
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Ref {
        unsafe { &*state.add(index) }
    }

    #[inline]
    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Self::at(state, index)
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<T>().segment(self.segment())]
    }
}

impl<T: Send + Sync + 'static> AsRef<T> for Read<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}
