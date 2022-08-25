use crate::{
    depend::Dependency,
    error::Result,
    inject::{Adapt, Context, Get},
    meta::Meta,
    store::Store,
    Inject,
};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct Write<T>(Arc<Store>, PhantomData<T>);
pub struct Read<T>(Write<T>);

pub trait Resource: Default + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
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

    #[inline]
    pub fn read(&self) -> Read<T> {
        Read(Self(self.0.clone(), PhantomData))
    }
}

unsafe impl<R: Resource> Inject for &mut R {
    type Input = <Write<R> as Inject>::Input;
    type State = <Write<R> as Inject>::State;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Write::<R>::initialize(input, context)
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        Write::<R>::depend(state)
    }
}

unsafe impl<R: Resource> Inject for Write<R> {
    type Input = Option<R>;
    type State = Self;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        let resources = context.world().resources();
        let store = unsafe { resources.get_store::<R, _>(|| input.unwrap_or_else(R::default)) };
        Ok(Self(store, PhantomData))
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![
            Dependency::write::<R>(),
            Dependency::write_at(state.store().identifier()),
        ]
    }
}

impl<'a, R: Resource> Get<'a> for Write<R> {
    type Item = &'a mut R;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        &mut *self.store().data()
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

unsafe impl<R: Resource> Inject for &R {
    type Input = <Read<R> as Inject>::Input;
    type State = <Read<R> as Inject>::State;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Read::<R>::initialize(input, context)
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        Read::<R>::depend(state)
    }
}

unsafe impl<R: Resource> Inject for Read<R> {
    type Input = <Write<R> as Inject>::Input;
    type State = Self;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Write::<R>::initialize(input, context.map(|Self(state)| state)).map(Self)
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![
            Dependency::read::<R>(),
            Dependency::read_at(state.store().identifier()),
        ]
    }
}

impl<'a, R: Resource> Get<'a> for Read<R> {
    type Item = &'a R;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        &*self.store().data()
    }
}

impl<T: Send + Sync + 'static> Deref for Read<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
