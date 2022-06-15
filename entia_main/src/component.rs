use crate::{
    depend::{Depend, Dependency},
    error::Result,
    meta::Meta,
    query::item::{self, At, Item},
    store::Store,
    world::World,
};
use std::{marker::PhantomData, sync::Arc};

pub struct Write<T>(Arc<Store>, usize, PhantomData<T>);
pub struct Read<T>(Write<T>);

pub trait Component: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}

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

impl<'a, T: Send + Sync + 'static> At<'a> for Write<T> {
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

impl<T: Send + Sync + 'static> AsRef<T> for Write<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}

impl<T: Send + Sync + 'static> AsMut<T> for Write<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.get(0) }
    }
}

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
