use std::{any::TypeId, sync::Arc};

use crate::{
    component::Component,
    inject::{Get, Inject},
    item::{At, Item},
    resource::{initialize, Resource},
    segment::Segment,
    system::Dependency,
    world::Store,
    world::World,
};

pub struct Write<T>(Arc<Store<T>>);
pub struct State<T>(Arc<Store<T>>, usize);

impl<R: Resource> Inject for Write<R> {
    type Input = Option<R>;
    type State = State<R>;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        initialize(|| input.unwrap_or_default(), world).map(|pair| State(pair.0, pair.1))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<R>())]
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

    fn initialize(segment: &Segment, _: &World) -> Option<Self::State> {
        Some(State(segment.static_store()?, segment.index))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<C>())]
    }
}

impl<'a, C: Component> At<'a> for State<C> {
    type Item = &'a mut C;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        unsafe { self.0.at(index) }
    }
}

impl<'a, R: Resource> From<&'a mut State<R>> for Write<R> {
    #[inline]
    fn from(state: &'a mut State<R>) -> Self {
        Write(state.0.clone())
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
