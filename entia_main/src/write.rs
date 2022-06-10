use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{meta::Meta, store::Store, Component, Resource, World},
};
use std::{marker::PhantomData, sync::Arc};

pub struct Write<T>(Arc<Store>, usize, PhantomData<T>);

impl<T> Write<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.1
    }

    #[inline]
    pub fn store(&self) -> &Store {
        &self.0
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        self.store().meta()
    }
}

impl<R: Resource> Inject for &mut R {
    type Input = <Write<R> as Inject>::Input;
    type State = <Write<R> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Write<_> as Inject>::initialize(input, context)
    }
}

impl<R: Resource> Inject for Write<R> {
    type Input = Option<R>;
    type State = Self;

    fn initialize(input: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let store =
            context
                .world()
                .get_or_add_resource_store(R::meta, |meta, world| match input {
                    Some(resource) => Ok(resource),
                    None => R::initialize(meta, world),
                })?;
        Ok(Self(store, usize::MAX, PhantomData))
    }
}

impl<'a, T: 'static> Get<'a> for Write<T> {
    type Item = <Self as At<'a>>::Mut;

    #[inline]
    fn get(&'a mut self, world: &World) -> Self::Item {
        Self::at_mut(&mut At::get(self, world), 0)
    }
}

impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Write::<C>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Write<T> {
    type State = Self;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<T>()?;
        let segment = context.segment();
        let store = segment.component_store(&meta)?;
        Ok(Self(store, segment.index(), PhantomData))
    }
}

impl<'a, T: 'static> At<'a> for Write<T> {
    type State = *mut T;
    type Ref = &'a T;
    type Mut = &'a mut T;

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
        unsafe { &mut *state.add(index) }
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>().segment(self.1)]
    }
}

impl<T: 'static> AsRef<T> for Write<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}

impl<T: 'static> AsMut<T> for Write<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.get(0) }
    }
}
