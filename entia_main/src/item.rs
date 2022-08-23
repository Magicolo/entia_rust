use crate::{
    depend::Dependency,
    error::Result,
    inject::{Adapt, Context},
    segment::Segment,
    tuples_with,
};
use std::marker::PhantomData;

pub trait Item {
    type State: for<'a> At<'a> + Send + Sync + 'static;
    fn initialize<A: Adapt<Self::State>>(
        segment: &Segment,
        context: Context<Self::State, A>,
    ) -> Result<Self::State>;
    fn depend(state: &Self::State) -> Vec<Dependency>;
}

pub trait At<'a, I = usize> {
    type State;
    type Ref;
    type Mut;

    fn get(&'a self, segment: &Segment) -> Option<Self::State>;
    unsafe fn at_ref(state: &Self::State, index: I) -> Self::Ref;
    unsafe fn at_mut(state: &mut Self::State, index: I) -> Self::Mut;
}

impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize<A: Adapt<Self::State>>(
        segment: &Segment,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Ok(I::initialize(segment, context.flat_map(Option::as_mut)).ok())
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        state.iter().flat_map(I::depend).collect()
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

    fn initialize<A: Adapt<Self::State>>(
        segment: &Segment,
        context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        <() as Item>::initialize(segment, context)
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        <() as Item>::depend(state)
    }
}

macro_rules! item {
    ($n:ident, $c:tt $(, $p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize<A: Adapt<Self::State>>(
                _segment: &Segment,
                mut _context: Context<Self::State, A>,
            ) -> Result<Self::State> {
                Ok(($($t::initialize(_segment, _context.map(|state| &mut state.$i))?,)*))
            }

            fn depend(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.extend($t::depend($p));)*
                _dependencies
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

tuples_with!(item);
