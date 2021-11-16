use crate::{
    depend::Depend,
    error::Result,
    inject::{Context, Get, Inject},
    world::World,
    write::Write,
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub struct Local<'a, T>(&'a mut T);

pub struct State<T> {
    index: usize,
    inner: Write<Inner>,
    _marker: PhantomData<T>,
}

#[derive(Default)]
struct Inner {
    states: Vec<Box<dyn Any + Send + Sync>>,
    indices: HashMap<(usize, TypeId), usize>,
}

impl<T: Default + Send + Sync + 'static> Inject for Local<'_, T> {
    type Input = Option<T>;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: Context) -> Result<Self::State> {
        let mut inner = <Write<Inner> as Inject>::initialize(None, context.owned())?;
        let index = {
            let key = (context.identifier(), TypeId::of::<T>());
            let inner = inner.as_mut();
            match inner.indices.get(&key) {
                Some(&index) => index,
                None => {
                    let index = inner.states.len();
                    inner.states.push(Box::new(input.unwrap_or_default()));
                    index
                }
            }
        };
        Ok(State {
            index,
            inner,
            _marker: PhantomData,
        })
    }
}

impl<'a, T: Default + 'static> Get<'a> for State<T> {
    type Item = Local<'a, T>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Local(
            self.inner.get(world).states[self.index]
                .downcast_mut()
                .unwrap(),
        )
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
        self.inner.as_ref().states[self.index]
            .downcast_ref()
            .unwrap()
    }
}

impl<T: 'static> AsMut<T> for State<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.inner.as_mut().states[self.index]
            .downcast_mut()
            .unwrap()
    }
}

impl<T> Deref for Local<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Local<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> AsRef<T> for Local<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.0
    }
}

impl<T> AsMut<T> for Local<'_, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.0
    }
}
