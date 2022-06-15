use crate::{
    depend::{Depend, Dependency},
    error::{Error, Result},
    inject::{self, Get},
    meta::Meta,
    store::Store,
    world::World,
    Inject,
};
use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
    sync::Arc,
};

pub struct Write<T>(Arc<Store>, PhantomData<T>);
pub struct Read<T>(Write<T>);

pub trait Resource: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }

    fn initialize(meta: &Meta, _: &mut World) -> Result<Self> {
        match meta.default() {
            Some(resource) => Ok(resource),
            None => Err(Error::MissingResource {
                name: type_name::<Self>(),
                identifier: TypeId::of::<Self>(),
            }),
        }
    }
}

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
        let store =
            context
                .world()
                .get_or_add_resource_store(R::meta, |meta, world| match input {
                    Some(resource) => Ok(resource),
                    None => R::initialize(meta, world),
                })?;
        Ok(Self(store, PhantomData))
    }
}

impl<'a, T: Send + Sync + 'static> Get<'a> for Write<T> {
    type Item = &'a mut T;

    #[inline]
    fn get(&'a mut self, world: &World) -> Self::Item {
        unsafe { &mut *self.store().data() }
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>().segment(usize::MAX)]
    }
}

impl<T: Send + Sync + 'static> AsRef<T> for Write<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.store().data() }
    }
}

impl<T: Send + Sync + 'static> AsMut<T> for Write<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
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

impl<'a, T: Send + Sync + 'static> Get<'a> for Read<T> {
    type Item = &'a T;

    #[inline]
    fn get(&'a mut self, world: &World) -> Self::Item {
        unsafe { &*self.store().data() }
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<T>().segment(usize::MAX)]
    }
}

impl<T: Send + Sync + 'static> AsRef<T> for Read<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.store().data() }
    }
}
