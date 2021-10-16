use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    query::item::{At, Item, ItemContext},
    resource::initialize,
    world::{store::Store, World},
    Resource,
};

pub struct Read<T>(Arc<Store>, PhantomData<T>);
pub struct State<T>(Arc<Store>, usize, PhantomData<T>);

unsafe impl<T: Default + 'static> Inject for Read<T> {
    type Input = Option<T>;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: InjectContext) -> Option<Self::State> {
        let (store, segment) = initialize(input, context.world())?;
        Some(State(store, segment, PhantomData))
    }
}

impl<'a, T: 'static> Get<'a> for State<T> {
    type Item = &'a T;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.get(0) }
    }
}

unsafe impl<T: Send + 'static> Item for Read<T> {
    type State = State<T>;

    fn initialize(mut context: ItemContext) -> Option<Self::State> {
        let meta = context.world().get_meta::<T>()?;
        let segment = context.segment();
        let store = segment.store(&meta)?;
        Some(State(store, segment.index, PhantomData))
    }
}

impl<'a, T: 'static> At<'a> for State<T> {
    type Item = &'a T;

    #[inline]
    fn at(&'a self, index: usize, _: &'a World) -> Self::Item {
        unsafe { self.0.get(index) }
    }
}

impl<T> State<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.1
    }

    #[inline]
    pub fn store(&self) -> &Store {
        &self.0
    }
}

impl<T> Clone for State<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1, PhantomData)
    }
}

unsafe impl<T: 'static> Depend for State<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(self.1, TypeId::of::<T>())]
    }
}

impl<'a, T> From<&'a State<T>> for Read<T> {
    #[inline]
    fn from(state: &'a State<T>) -> Self {
        Read(state.0.clone(), PhantomData)
    }
}

impl<'a, T> From<&'a mut State<T>> for Read<T> {
    #[inline]
    fn from(state: &'a mut State<T>) -> Self {
        Read(state.0.clone(), PhantomData)
    }
}

impl<R: Resource> AsRef<R> for Read<R> {
    #[inline]
    fn as_ref(&self) -> &R {
        unsafe { self.0.get(0) }
    }
}

impl<R: Resource> AsRef<R> for State<R> {
    #[inline]
    fn as_ref(&self) -> &R {
        unsafe { self.0.get(0) }
    }
}
