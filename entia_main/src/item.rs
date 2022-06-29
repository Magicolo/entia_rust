use crate::{depend::Depend, error::Result, inject, recurse, segment::Segment, world::World};
use std::marker::PhantomData;

pub struct Context<'a> {
    identifier: usize,
    segment: usize,
    world: &'a mut World,
}

pub trait Item {
    type State: for<'a> Chunk<'a> + Depend;
    fn initialize(context: Context) -> Result<Self::State>;
}

pub trait Chunk<'a> {
    type Ref: At<'a>;
    type Mut: At<'a>;
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref>;
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut>;
}

pub trait At<'a, I = usize> {
    type Ref;
    type Mut;
    fn at(&'a self, index: I) -> Option<Self::Ref>;
    unsafe fn at_unchecked(&'a self, index: I) -> Self::Ref;
    fn at_mut(&'a mut self, index: I) -> Option<Self::Mut>;
    unsafe fn at_unchecked_mut(&'a mut self, index: I) -> Self::Mut;
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

impl<'a, C: Chunk<'a> + ?Sized> Chunk<'a> for &C {
    type Ref = C::Ref;
    type Mut = C::Mut;

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        C::chunk(self, segment)
    }
    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        C::chunk_mut(self, segment)
    }
}

impl<'a, C: Chunk<'a> + ?Sized> Chunk<'a> for &mut C {
    type Ref = C::Ref;
    type Mut = C::Mut;

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        C::chunk(self, segment)
    }
    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        C::chunk_mut(self, segment)
    }
}

impl<'a, I, A: At<'a, I> + ?Sized> At<'a, I> for &A {
    type Ref = A::Ref;
    type Mut = Self::Ref;

    #[inline]
    fn at(&'a self, index: I) -> Option<Self::Ref> {
        A::at(self, index)
    }
    #[inline]
    unsafe fn at_unchecked(&'a self, index: I) -> Self::Ref {
        A::at_unchecked(self, index)
    }
    #[inline]
    fn at_mut(&'a mut self, index: I) -> Option<Self::Mut> {
        Self::at(self, index)
    }
    #[inline]
    unsafe fn at_unchecked_mut(&'a mut self, index: I) -> Self::Mut {
        Self::at_unchecked(self, index)
    }
}

impl<'a, I, A: At<'a, I> + ?Sized> At<'a, I> for &mut A {
    type Ref = A::Ref;
    type Mut = A::Mut;

    #[inline]
    fn at(&'a self, index: I) -> Option<Self::Ref> {
        A::at(self, index)
    }
    #[inline]
    unsafe fn at_unchecked(&'a self, index: I) -> Self::Ref {
        A::at_unchecked(self, index)
    }
    #[inline]
    fn at_mut(&'a mut self, index: I) -> Option<Self::Mut> {
        A::at_mut(self, index)
    }
    #[inline]
    unsafe fn at_unchecked_mut(&'a mut self, index: I) -> Self::Mut {
        A::at_unchecked_mut(self, index)
    }
}

impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize(context: Context) -> Result<Self::State> {
        Ok(I::initialize(context).ok())
    }
}

impl<'a, C: Chunk<'a>> Chunk<'a> for Option<C> {
    type Ref = Option<C::Ref>;
    type Mut = Option<C::Mut>;

    #[inline]
    fn chunk(&'a self, segment: &'a Segment) -> Option<Self::Ref> {
        Some(match self {
            Some(chunk) => Some(chunk.chunk(segment)?),
            None => None,
        })
    }

    #[inline]
    fn chunk_mut(&'a self, segment: &'a Segment) -> Option<Self::Mut> {
        Some(match self {
            Some(chunk) => Some(chunk.chunk_mut(segment)?),
            None => None,
        })
    }
}

impl<'a, I, A: At<'a, I>> At<'a, I> for Option<A> {
    type Ref = Option<A::Ref>;
    type Mut = Option<A::Mut>;

    #[inline]
    fn at(&'a self, index: I) -> Option<Self::Ref> {
        Some(match self {
            Some(at) => Some(at.at(index)?),
            None => None,
        })
    }

    #[inline]
    unsafe fn at_unchecked(&'a self, index: I) -> Self::Ref {
        match self {
            Some(at) => Some(at.at_unchecked(index)),
            None => None,
        }
    }

    #[inline]
    fn at_mut(&'a mut self, index: I) -> Option<Self::Mut> {
        Some(match self {
            Some(at) => Some(at.at_mut(index)?),
            None => None,
        })
    }

    #[inline]
    unsafe fn at_unchecked_mut(&'a mut self, index: I) -> Self::Mut {
        match self {
            Some(at) => Some(at.at_unchecked_mut(index)),
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

pub struct TupleIterator<T>(T);
macro_rules! item {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(mut _context: Context) -> Result<Self::State> {
                Ok(($($t::initialize(_context.owned())?,)*))
            }
        }

        impl<'a, $($t: Chunk<'a>,)*> Chunk<'a> for ($($t,)*) {
            type Ref = ($($t::Ref,)*);
            type Mut = ($($t::Mut,)*);

            #[inline]
            fn chunk(&'a self, _segment: &'a Segment) -> Option<Self::Ref> {
                let ($($p,)*) = self;
                Some(($($p.chunk(_segment)?,)*))
            }

            #[inline]
            fn chunk_mut(&'a self, _segment: &'a Segment) -> Option<Self::Mut> {
                let ($($p,)*) = self;
                Some(($($p.chunk_mut(_segment)?,)*))
            }
        }

        impl<'a, I: Clone, $($t: At<'a, I>,)*> At<'a, I> for ($($t,)*) {
            type Ref = ($($t::Ref,)*);
            type Mut = ($($t::Mut,)*);

            #[inline]
            fn at(&'a self, _index: I) -> Option<Self::Ref> {
                let ($($p,)*) = self;
                Some(($($p.at(_index.clone())?,)*))
            }

            #[inline]
            unsafe fn at_unchecked(&'a self, _index: I) -> Self::Ref {
                let ($($p,)*) = self;
                ($($p.at_unchecked(_index.clone()),)*)
            }

            #[inline]
            fn at_mut(&'a mut self, _index: I) -> Option<Self::Mut> {
                let ($($p,)*) = self;
                Some(($($p.at_mut(_index.clone())?,)*))
            }

            #[inline]
            unsafe fn at_unchecked_mut(&'a mut self, _index: I) -> Self::Mut {
                let ($($p,)*) = self;
                ($($p.at_unchecked_mut(_index.clone()),)*)
            }
        }
    };
}

recurse!(item);
