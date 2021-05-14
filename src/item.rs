use crate::system::*;
use crate::world::*;
use std::marker::PhantomData;

pub struct And<I: Item>(PhantomData<I>);
pub struct Not<I: Item>(PhantomData<I>);

pub trait Item {
    type State: for<'a> At<'a> + 'static;
    fn initialize(segment: &Segment) -> Option<Self::State>;
    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

pub trait At<'a> {
    type Item;
    fn at(&'a self, index: usize) -> Self::Item;
}

pub struct DefaultState<T: Default>(PhantomData<T>);

impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(I::initialize(segment))
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        match state {
            Some(state) => I::depend(state, world),
            None => Vec::new(),
        }
    }
}

impl<'a, A: At<'a>> At<'a> for Option<A> {
    type Item = Option<A::Item>;

    #[inline]
    fn at(&'a self, index: usize) -> Self::Item {
        self.as_ref().map(|value| value.at(index))
    }
}

impl<T: Default> At<'_> for DefaultState<T> {
    type Item = T;

    #[inline]
    fn at(&self, _: usize) -> Self::Item {
        T::default()
    }
}

impl<I: Item> Default for And<I> {
    #[inline]
    fn default() -> Self {
        And(PhantomData)
    }
}

impl<I: Item + 'static> Item for And<I> {
    type State = DefaultState<Self>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        match I::initialize(segment) {
            Some(_) => Some(DefaultState(PhantomData)),
            None => None,
        }
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<I: Item> Default for Not<I> {
    #[inline]
    fn default() -> Self {
        Not(PhantomData)
    }
}

impl<I: Item + 'static> Item for Not<I> {
    type State = DefaultState<Self>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        match I::initialize(segment) {
            Some(_) => None,
            None => Some(DefaultState(PhantomData)),
        }
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

macro_rules! item {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment) -> Option<Self::State> {
                Some(($($t::initialize(_segment)?,)*))
            }

            fn depend(($($p,)*): &Self::State, _world: &World) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::depend($p, _world));)*
                _dependencies
            }
        }

        impl<'a, $($t: At<'a>,)*> At<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            fn at(&'a self, _index: usize) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.at(_index),)*)
            }
        }
    };
}

crate::recurse!(
    item, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10, T10, p11,
    T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18, p19, T19, p20, T20,
    p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27, T27, p28, T28, p29, T29, p30,
    T30, p31, T31, p32, T32
);
