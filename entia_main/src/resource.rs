use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::Get,
    meta::{Describe, Meta},
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

pub trait Resource: Default + Describe {}
impl<T: Default + Describe> Resource for T {}

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

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        <Write<_> as Inject>::initialize(input, identifier, world)
    }
}

impl<R: Resource> Inject for Write<R> {
    type Input = Option<R>;
    type State = Self;

    fn initialize(input: Self::Input, _: usize, world: &mut World) -> Result<Self::State> {
        let resources = world.resources();
        let store = unsafe { resources.get_store::<R>(|| input.unwrap_or_else(R::default)) };
        Ok(Self(store, PhantomData))
    }
}

impl<'a, R: Resource> Get<'a> for Write<R> {
    type Item = &'a mut R;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        &mut *self.store().data()
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self) -> Vec<Dependency> {
        vec![Dependency::write::<T>(self.store().identifier())]
    }
}

impl<T> Into<Read<T>> for Write<T> {
    #[inline]
    fn into(self) -> Read<T> {
        Read(self)
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

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        <Read<_> as Inject>::initialize(input, identifier, world)
    }
}

impl<R: Resource> Inject for Read<R> {
    type Input = <Write<R> as Inject>::Input;
    type State = Self;

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        <Write<_> as Inject>::initialize(input, identifier, world).map(Read)
    }
}

impl<'a, R: Resource> Get<'a> for Read<R> {
    type Item = &'a R;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        &*self.store().data()
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self) -> Vec<Dependency> {
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
