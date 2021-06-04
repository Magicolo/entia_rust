use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{
    component::Component,
    depend::{Depend, Dependency},
    inject::{Context, Get, Inject},
    item::{At, Item},
    resource::{initialize, Resource},
    segment::Segment,
    segment::Store,
    world::World,
};

pub struct Write<T>(Arc<Store>, PhantomData<T>);
pub struct State<T>(Arc<Store>, usize, PhantomData<T>);

impl<R: Resource> Inject for Write<R> {
    type Input = Option<R>;
    type State = State<R>;

    fn initialize(input: Self::Input, _: &Context, world: &mut World) -> Option<Self::State> {
        let (store, segment) = initialize(|| input.unwrap_or_default(), world)?;
        Some(State(store, segment, PhantomData))
    }
}

impl<'a, R: Resource> Get<'a> for State<R> {
    type Item = &'a mut R;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}

impl<C: Component> Item for Write<C> {
    type State = State<C>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        let meta = world.get_meta::<C>()?;
        let store = segment.store(&meta)?;
        Some(State(store, segment.index, PhantomData))
    }
}

impl<'a, C: Component> At<'a> for State<C> {
    type Item = &'a mut C;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        unsafe { self.0.at(index) }
    }
}

impl<T: 'static> Depend for State<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(self.1, TypeId::of::<T>())]
    }
}

impl<'a, R: Resource> From<&'a mut State<R>> for Write<R> {
    #[inline]
    fn from(state: &'a mut State<R>) -> Self {
        Write(state.0.clone(), PhantomData)
    }
}

impl<R: Resource> AsRef<R> for Write<R> {
    #[inline]
    fn as_ref(&self) -> &R {
        unsafe { self.0.at(0) }
    }
}

impl<R: Resource> AsMut<R> for Write<R> {
    #[inline]
    fn as_mut(&mut self) -> &mut R {
        unsafe { self.0.at(0) }
    }
}

impl<R: Resource> AsRef<R> for State<R> {
    #[inline]
    fn as_ref(&self) -> &R {
        unsafe { self.0.at(0) }
    }
}

impl<R: Resource> AsMut<R> for State<R> {
    #[inline]
    fn as_mut(&mut self) -> &mut R {
        unsafe { self.0.at(0) }
    }
}
