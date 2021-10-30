use std::{marker::PhantomData, sync::Arc};

use crate::{
    depend::{Depend, Dependency},
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{store::Store, World},
};

pub struct Read<T>(Arc<Store>, PhantomData<T>);
pub struct State<T>(Arc<Store>, usize, PhantomData<T>);

impl<T: Default + Send + Sync + 'static> Inject for &T {
    type Input = <Read<T> as Inject>::Input;
    type State = <Read<T> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Option<Self::State> {
        <Read<T> as Inject>::initialize(input, context)
    }
}

impl<T: Default + Send + Sync + 'static> Inject for Read<T> {
    type Input = Option<T>;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: inject::Context) -> Option<Self::State> {
        let (store, segment) = context.world().initialize(input)?;
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

impl<T: Send + Sync + 'static> Item for &T {
    type State = <Read<T> as Item>::State;

    fn initialize(context: item::Context) -> Option<Self::State> {
        <Read<T> as Item>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Read<T> {
    type State = State<T>;

    fn initialize(mut context: item::Context) -> Option<Self::State> {
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
        vec![Dependency::read::<T>().at(self.1)]
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

impl<T: 'static> AsRef<T> for Read<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}

impl<T: 'static> AsRef<T> for State<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}
