use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{self, Get},
    meta::Meta,
    store::Store,
    world::World,
    Inject,
};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct Write<T>(Arc<Store>, PhantomData<T>);
pub struct Read<T>(Write<T>);

pub trait Resource: Sized + Default + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}
impl<T: Default + Send + Sync + 'static> Resource for T {}

impl<T> Write<T> {
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
        let store = context
            .world()
            .get_or_add_resource_store(R::meta, || input.unwrap_or_else(R::default));
        Ok(Self(store, PhantomData))
    }
}

impl<'a, R: Resource> Get<'a> for Write<R> {
    type Item = &'a mut R;

    #[inline]
    unsafe fn get(&'a mut self, _: &World) -> Self::Item {
        &mut *self.store().data()
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>(self.store().identifier())]
    }
}

impl<T: Send + Sync + 'static> Deref for Write<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.store().data() }
    }
}

impl<T: Send + Sync + 'static> DerefMut for Write<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.store().data() }
    }
}

impl<T> Read<T> {
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

impl<'a, R: Resource> Get<'a> for Read<R> {
    type Item = &'a R;

    #[inline]
    unsafe fn get(&'a mut self, _: &World) -> Self::Item {
        &*self.store().data()
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<T>(self.store().identifier())]
    }
}

impl<T: Send + Sync + 'static> Deref for Read<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
