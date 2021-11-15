use std::{marker::PhantomData, sync::Arc};

use crate::{
    depend::{Depend, Dependency},
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{store::Store, World},
    Result,
};

pub struct Write<T>(Arc<Store>, PhantomData<T>);
pub struct State<T>(Arc<Store>, usize, PhantomData<T>);

impl<T: Default + Send + Sync + 'static> Inject for &mut T {
    type Input = <Write<T> as Inject>::Input;
    type State = <Write<T> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Write<T> as Inject>::initialize(input, context)
    }
}

impl<T: Default + Send + Sync + 'static> Inject for Write<T> {
    type Input = Option<T>;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let (store, segment) = context.world().initialize(input)?;
        Ok(State(store, segment, PhantomData))
    }
}

impl<'a, T: 'static> Get<'a> for State<T> {
    type Item = &'a mut T;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.get(0) }
    }
}

impl<T: Send + Sync + 'static> Item for &mut T {
    type State = <Write<T> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Write<T> as Item>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Write<T> {
    type State = State<T>;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<T>()?;
        let segment = context.segment();
        let store = segment.store(&meta)?;
        Ok(State(store, segment.index(), PhantomData))
    }
}

impl<'a, T: 'static> At<'a> for State<T> {
    type Item = &'a mut T;

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
        vec![Dependency::write::<T>().at(self.1)]
    }
}

impl<'a, T> From<&'a mut State<T>> for Write<T> {
    #[inline]
    fn from(state: &'a mut State<T>) -> Self {
        Write(state.0.clone(), PhantomData)
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

impl<T: 'static> AsRef<T> for State<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.get(0) }
    }
}

impl<T: 'static> AsMut<T> for State<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.get(0) }
    }
}
