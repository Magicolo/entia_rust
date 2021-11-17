use crate::{
    depend::{Depend, Dependency},
    error::Result,
    inject::{self, Get, Inject},
    query::item::{self, At, Item},
    world::{segment::Column, World},
};
use std::marker::PhantomData;

pub struct Write<T>(Column, usize, PhantomData<T>);

impl<T> Write<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.1
    }
}

impl<T: Default + Send + Sync + 'static> Inject for &mut T {
    type Input = <Write<T> as Inject>::Input;
    type State = <Write<T> as Inject>::State;

    fn initialize(input: Self::Input, context: inject::Context) -> Result<Self::State> {
        <Write<_> as Inject>::initialize(input, context)
    }
}

impl<T: Default + Send + Sync + 'static> Inject for Write<T> {
    type Input = Option<T>;
    type State = Self;

    fn initialize(input: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let (column, segment) = context.world().get_or_add_resource(input);
        Ok(Self(column, segment, PhantomData))
    }
}

impl<'a, T: 'static> Get<'a> for Write<T> {
    type Item = &'a mut T;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.store().get(0) }
    }
}

impl<T: Send + Sync + 'static> Item for &mut T {
    type State = <Write<T> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Write::<T>::initialize(context)
    }
}

impl<T: Send + Sync + 'static> Item for Write<T> {
    type State = Self;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<T>()?;
        let segment = context.segment();
        let column = segment.column(&meta)?;
        Ok(Self(column, segment.index(), PhantomData))
    }
}

impl<'a, T: 'static> At<'a> for Write<T> {
    type Item = &'a mut T;

    #[inline]
    fn at(&'a self, index: usize, _: &'a World) -> Self::Item {
        unsafe { self.0.store().get(index) }
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>().at(self.1)]
    }
}

impl<T: 'static> AsRef<T> for Write<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.0.store().get(0) }
    }
}

impl<T: 'static> AsMut<T> for Write<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.store().get(0) }
    }
}
