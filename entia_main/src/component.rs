use crate::{
    depend::{Depend, Dependency},
    error::Result,
    item::{self, At, Chunk, Item},
    meta::Meta,
    segment::Segment,
    store::Store,
    world::World,
};
use std::{marker::PhantomData, slice::SliceIndex, sync::Arc};

pub struct Write<T>(Arc<Store>, usize, PhantomData<T>);
pub struct Read<T>(Write<T>);

pub trait Component: Sized + Send + Sync + 'static {
    fn meta() -> Meta {
        crate::meta!(Self)
    }
}

impl<'a, C: Component, I: SliceIndex<[C]>> At<'a, I> for [C]
where
    I::Output: 'a,
{
    type Ref = &'a I::Output;
    type Mut = &'a mut I::Output;

    #[inline]
    fn at(&'a self, index: I) -> Option<Self::Ref> {
        self.get(index)
    }

    #[inline]
    unsafe fn at_unchecked(&'a self, index: I) -> Self::Ref {
        self.get_unchecked(index)
    }

    #[inline]
    fn at_mut(&'a mut self, index: I) -> Option<Self::Mut> {
        self.get_mut(index)
    }

    #[inline]
    unsafe fn at_unchecked_mut(&'a mut self, index: I) -> Self::Mut {
        self.get_unchecked_mut(index)
    }
}

impl<T> Write<T> {
    #[inline]
    pub const fn segment(&self) -> usize {
        self.1
    }

    #[inline]
    pub fn store(&self) -> &Store {
        &self.0
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        self.store().meta()
    }
}

impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        Write::<C>::initialize(context)
    }
}

impl<C: Component> Item for Write<C> {
    type State = Self;

    fn initialize(mut context: item::Context) -> Result<Self::State> {
        let meta = context.world().get_meta::<C>()?;
        let segment = context.segment();
        let store = segment.component_store(&meta)?;
        Ok(Self(store, segment.index(), PhantomData))
    }
}

impl<'a, C: Component> Chunk<'a> for Write<C> {
    type Ref = &'a [C];
    type Mut = &'a mut [C];

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        debug_assert_eq!(self.segment(), segment.index());
        Some(unsafe { self.store().get_all(segment.count()) })
    }

    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        debug_assert_eq!(self.segment(), segment.index());
        Some(unsafe { self.store().get_all(segment.count()) })
    }
}

unsafe impl<T: 'static> Depend for Write<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::write::<T>().segment(self.1)]
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

impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Read<_> as Item>::initialize(context)
    }
}

impl<C: Component> Item for Read<C> {
    type State = Self;

    fn initialize(context: item::Context) -> Result<Self::State> {
        <Write<_> as Item>::initialize(context).map(Read)
    }
}

impl<'a, C: Component> Chunk<'a> for Read<C> {
    type Ref = <Write<C> as Chunk<'a>>::Ref;
    type Mut = Self::Ref;

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        self.0.chunk(segment)
    }

    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        self.0.chunk(segment)
    }
}

unsafe impl<T: 'static> Depend for Read<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::read::<T>().segment(self.segment())]
    }
}
