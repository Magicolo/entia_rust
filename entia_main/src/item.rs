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

pub trait At<'a> {
    type State;
    type Ref;
    type Mut;
    fn get(&'a self, world: &'a World) -> Self::State;
    fn at(state: &Self::State, index: usize) -> Self::Ref;
    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut;
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

impl<'a, A: At<'a>> At<'a> for Option<A> {
    type State = Option<A::State>;
    type Ref = Option<A::Ref>;
    type Mut = Option<A::Mut>;

    #[inline]
    fn get(&'a self, world: &'a World) -> Self::State {
        Some(self.as_ref()?.get(world))
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Ref {
        Some(A::at(state.as_ref()?, index))
    }

    #[inline]
    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Some(A::at_mut(state.as_mut()?, index))
    }
}

impl<T> Item for PhantomData<T> {
    type State = <() as Item>::State;
    fn initialize(context: Context) -> Result<Self::State> {
        <() as Item>::initialize(context)
    }
}

impl<'a, T> At<'a> for PhantomData<T> {
    type State = <() as At<'a>>::State;
    type Ref = <() as At<'a>>::Ref;
    type Mut = <() as At<'a>>::Mut;

    #[inline]
    fn get(&'a self, world: &'a World) -> Self::State {
        ().get(world)
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Ref {
        <()>::at(state, index)
    }

    #[inline]
    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        <()>::at_mut(state, index)
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

        impl<'a, $($t: At<'a>,)*> At<'a> for ($($t,)*) {
            type State = ($($t::State,)*);
            type Ref = ($($t::Ref,)*);
            type Mut = ($($t::Mut,)*);

            #[inline]
            fn get(&'a self, _world: &'a World) -> Self::State {
                let ($($p,)*) = self;
                ($($p.get(_world),)*)
            }

            #[inline]
            fn at(_state: &Self::State, _index: usize) -> Self::Ref {
                let ($($p,)*) = _state;
                ($($t::at($p, _index),)*)
            }

            #[inline]
            fn at_mut(_state: &mut Self::State, _index: usize) -> Self::Mut {
                let ($($p,)*) = _state;
                ($($t::at_mut($p, _index),)*)
            }
        }
    };
}

recurse!(item);
