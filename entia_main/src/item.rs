use crate::{depend::Depend, error::Result, inject, recurse, segment::Segment, world::World};
use std::marker::PhantomData;

pub struct Context<'a> {
    identifier: usize,
    segment: usize,
    world: &'a mut World,
}

pub trait Item {
    type State: for<'a> At<'a> + Depend;
    fn initialize(context: Context) -> Result<Self::State>;
}

pub trait At<'a, I = usize> {
    type State;
    type Ref;
    type Mut;

    fn get(&'a self, segment: &Segment) -> Option<Self::State>;
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref;
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut;
}

impl<'a> Context<'a> {
    pub fn new(identifier: usize, segment: usize, world: &'a mut World) -> Self {
        Self {
            identifier,
            segment,
            world,
        }
    }

    pub fn identifier(&self) -> usize {
        self.identifier
    }

    pub fn segment(&self) -> &Segment {
        &self.world.segments()[self.segment]
    }

    pub fn world(&mut self) -> &mut World {
        self.world
    }

    pub fn owned(&mut self) -> Context {
        self.with(self.segment)
    }

    pub fn with(&mut self, segment: usize) -> Context {
        Context::new(self.identifier, segment, self.world)
    }
}

impl<'a> Into<inject::Context<'a>> for Context<'a> {
    fn into(self) -> inject::Context<'a> {
        inject::Context::new(self.identifier, self.world)
    }
}

impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize(context: Context) -> Result<Self::State> {
        Ok(I::initialize(context).ok())
    }
}

impl<'a, I, A: At<'a, I>> At<'a, I> for Option<A> {
    type State = Option<A::State>;
    type Ref = Option<A::Ref>;
    type Mut = Option<A::Mut>;

    #[inline]
    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        Some(match self {
            Some(at) => A::get(at, segment),
            None => None,
        })
    }

    #[inline]
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref {
        match state {
            Some(state) => Some(A::at_ref(state, index)),
            None => None,
        }
    }

    #[inline]
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut {
        match state {
            Some(state) => Some(A::at_mut(state, index)),
            None => None,
        }
    }
}

impl<T> Item for PhantomData<T> {
    type State = <() as Item>::State;
    fn initialize(context: Context) -> Result<Self::State> {
        <() as Item>::initialize(context)
    }
}

macro_rules! item {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(mut _context: Context) -> Result<Self::State> {
                Ok(($($t::initialize(_context.owned())?,)*))
            }
        }

        impl<'a, I: Clone, $($t: At<'a, I>,)*> At<'a, I> for ($($t,)*) {
            type State = ($($t::State,)*);
            type Ref = ($($t::Ref,)*);
            type Mut = ($($t::Mut,)*);

            #[inline]
            fn get(&'a self, _segment: &Segment) -> Option<Self::State> {
                let ($($p,)*) = self;
                Some(($($p.get(_segment)?,)*))
            }

            #[inline]
            unsafe fn at_ref(($($p,)*): &Self::State, _index: I) -> Self::Ref {
                ($($t::at_ref($p, _index.clone()),)*)
            }

            #[inline]
            unsafe fn at_mut(($($p,)*): &mut Self::State, _index: I) -> Self::Mut {
                ($($t::at_mut($p, _index.clone()),)*)
            }
        }
    };
}

recurse!(item);
