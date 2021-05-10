use crate::system::*;
use crate::world::*;
use std::marker::PhantomData;

pub struct And<I: Item>(PhantomData<I>);
pub struct Not<I: Item>(PhantomData<I>);

pub trait Item {
    type State: for<'a> At<'a> + 'static;
    fn initialize(segment: &Segment) -> Option<Self::State>;
    fn dependencies(_: &Self::State) -> Vec<Dependency> {
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

    fn dependencies(state: &Self::State) -> Vec<Dependency> {
        match state {
            Some(state) => I::dependencies(state),
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

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
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

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

macro_rules! matcher {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment) -> Option<Self::State> {
                Some(($($t::initialize(_segment)?,)*))
            }

            fn dependencies(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::dependencies($p));)*
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
    matcher, matcher0, Q0, matcher1, Q1, matcher2, Q2, matcher3, Q3, matcher4, Q4, matcher5, Q5,
    matcher6, Q6, matcher7, Q7, matcher8, Q8, matcher9, Q9, matcher10, Q10, matcher11, Q11,
    matcher12, Q12, matcher13, Q13, matcher14, Q14, matcher15, Q15
);
