use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{store::Store, Component, Resource, World},
};
use std::{marker::PhantomData, sync::Arc};

pub struct Read<T>(Arc<Store>, usize, PhantomData<T>);

impl<T> Read<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.1
    }
}

impl<R: Resource> Inject for &R {
    type Input = <Read<R> as Inject>::Input;
    type State = <Read<R> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Read<_> as Inject>::initialize(input, context)
    }
}

impl<T: Default + Send + Sync + 'static> Inject for Read<T> {
    type Input = Option<T>;
    type State = Self;

    fn initialize(input: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let store = context
            .world()
            .get_or_add_resource_store(|| input.unwrap_or_default());
        Ok(Self(store, usize::MAX, PhantomData))
    }
}

impl<'a, T: 'static> Get<'a> for Read<T> {
    type Item = <Self as At<'a>>::Mut;

    #[inline]
    fn get(&'a mut self, world: &World) -> Self::Item {
        Self::at(&mut At::get(self, world), 0)
    }
}

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Read::<C>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Read<T> {
    type State = Self;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<T>()?;
        let segment = context.segment();
        let store = segment.component_store(&meta)?;
        Ok(Self(store, segment.index(), PhantomData))
    }
}

impl<'a, T: 'static> At<'a> for Read<T> {
    type State = *const T;
    type Ref = &'a T;
    type Mut = Self::Ref;

    #[inline]
    fn get(&'a self, _: &'a World) -> Self::State {
        self.0.data()
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
        vec![Dependency::read::<T>().at(self.1)]
    }
}

impl<T: 'static> AsRef<T> for Read<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}
