use crate::{
    depend::{Depend, Dependency},
    error::Result,
    item::{self, At, Item},
    meta::Meta,
    segment::Segment,
    store::Store,
    world::World,
};
use std::{
    marker::PhantomData,
    slice::{from_raw_parts, from_raw_parts_mut, SliceIndex},
    sync::Arc,
};

pub struct Write<T> {
    store: Arc<Store>,
    segment: usize,
    _marker: PhantomData<T>,
}
pub struct Read<T>(Write<T>);

pub trait Component: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}
impl<T: Send + Sync + 'static> Component for T {}

impl<T> Write<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.segment
    }

    #[inline]
    pub fn store(&self) -> &Store {
        &self.store
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        self.store().meta()
    }
}

impl<C: Component> Item for Write<C> {
    type State = Self;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<C>()?;
        let segment = context.segment();
        let store = segment.component_store(&meta)?;
        Ok(Self {
            store,
            segment: segment.index(),
            _marker: PhantomData,
        })
    }
}

impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Write::<C>::initialize(context)
    }
}

impl<'a, C: Component, I: SliceIndex<[C]>> At<'a, I> for Write<C>
where
    I::Output: 'a,
{
    type State = (*mut C, usize);
    type Ref = &'a I::Output;
    type Mut = &'a mut I::Output;

    #[inline]
    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        debug_assert_eq!(self.segment(), segment.index());
        Some((self.store().data(), segment.count()))
    }

    #[inline]
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref {
        from_raw_parts(state.0, state.1).get_unchecked(index)
    }

    #[inline]
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut {
        from_raw_parts_mut(state.0, state.1).get_unchecked_mut(index)
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>(self.store().identifier()).at(self.segment())]
    }
}

impl<T> Read<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.0.segment()
    }

    #[inline]
    pub fn store(&self) -> &Store {
        self.0.store()
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        self.0.meta()
    }
}

impl<C: Component> Item for Read<C> {
    type State = Self;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Write<_> as Item>::initialize(context).map(Read)
    }
}

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Read<_> as Item>::initialize(context)
    }
}

impl<'a, C: Component, I: SliceIndex<[C]>> At<'a, I> for Read<C>
where
    I::Output: 'a,
{
    type State = <Write<C> as At<'a, I>>::State;
    type Ref = <Write<C> as At<'a, I>>::Ref;
    type Mut = Self::Ref;

    #[inline]
    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        <Write<C> as At<'a, I>>::get(&self.0, segment)
    }

    #[inline]
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref {
        <Write<C> as At<'a, I>>::at_ref(state, index)
    }

    #[inline]
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut {
        Self::at_ref(state, index)
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<T>(self.store().identifier()).at(self.segment())]
    }
}
