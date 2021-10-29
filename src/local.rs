use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

use crate::{
    depend::Depend,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
};

pub struct Local<'a, T>(&'a mut T);
pub struct State<T>(usize, write::State<Inner>, PhantomData<T>);

#[derive(Default)]
struct Inner {
    states: Vec<Box<dyn Any + Send>>,
    indices: HashMap<(usize, TypeId), usize>,
}

impl<T: Default + Send + 'static> Inject for Local<'_, T> {
    type Input = ();
    type State = State<T>;

    fn initialize(_: Self::Input, mut context: Context) -> Option<Self::State> {
        let mut inner = <Write<Inner> as Inject>::initialize(None, context.owned())?;
        let index = {
            let key = (context.identifier(), TypeId::of::<T>());
            let inner = inner.as_mut();
            match inner.indices.get(&key) {
                Some(&index) => index,
                None => {
                    let index = inner.states.len();
                    inner.states.push(Box::new(T::default()));
                    index
                }
            }
        };
        Some(State(index, inner, PhantomData))
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone(), PhantomData)
    }
}

impl<'a, T: Default + Send + 'static> Get<'a> for State<T> {
    type Item = Local<'a, T>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Local(self.1.get(world).states[self.0].downcast_mut().unwrap())
    }
}

unsafe impl<T> Depend for State<T> {
    fn depend(&self, _: &World) -> Vec<crate::depend::Dependency> {
        Vec::new()
    }
}

impl<T: 'static> AsRef<T> for State<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.1.as_ref().states[self.0].downcast_ref().unwrap()
    }
}

impl<T: 'static> AsMut<T> for State<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.1.as_mut().states[self.0].downcast_mut().unwrap()
    }
}

impl<'a, T: 'static> AsRef<T> for Local<'a, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.0
    }
}

impl<'a, T: 'static> AsMut<T> for Local<'a, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.0
    }
}
