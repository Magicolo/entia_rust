use crate::system::*;
use crate::world::*;
use std::marker::PhantomData;

pub struct And<'a, Q: Query<'a>>(PhantomData<&'a Q>);
pub struct Not<'a, Q: Query<'a>>(PhantomData<&'a Q>);

pub trait Query<'a>: 'a {
    type State: Sync + Send + 'a;
    fn initialize(segment: &'a Segment, world: &'a World) -> Option<Self::State>;
    fn query(index: usize, state: &Self::State) -> Self;
    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

impl<'a, T: 'a> Query<'a> for PhantomData<T> {
    type State = ();

    fn initialize(_: &Segment, _: &World) -> Option<Self::State> {
        Some(())
    }

    fn query(_: usize, _: &Self::State) -> Self {
        PhantomData
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<'a, Q: Query<'a>> Query<'a> for Option<Q> {
    type State = Option<Q::State>;

    fn initialize(segment: &'a Segment, world: &'a World) -> Option<Self::State> {
        Some(Q::initialize(segment, world))
    }

    #[inline]
    fn query(index: usize, state: &Self::State) -> Self {
        match state {
            Some(state) => Some(Q::query(index, state)),
            None => None,
        }
    }

    fn dependencies(state: &Self::State) -> Vec<Dependency> {
        match state {
            Some(state) => Q::dependencies(state),
            None => Vec::new(),
        }
    }
}

impl<'a, Q: Query<'a>> Query<'a> for And<'a, Q> {
    type State = ();

    fn initialize(segment: &'a Segment, world: &'a World) -> Option<Self::State> {
        match Q::initialize(segment, world) {
            Some(_) => Some(()),
            None => None,
        }
    }

    #[inline]
    fn query(_: usize, _: &Self::State) -> Self {
        And(PhantomData)
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<'a, Q: Query<'a>> Query<'a> for Not<'a, Q> {
    type State = ();

    fn initialize(segment: &'a Segment, world: &'a World) -> Option<Self::State> {
        match Q::initialize(segment, world) {
            Some(_) => None,
            None => Some(()),
        }
    }

    #[inline]
    fn query(_: usize, _: &Self::State) -> Self {
        Not(PhantomData)
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

macro_rules! query {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Query<'a>,)*> Query<'a> for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(segment: &'a Segment, world: &'a World) -> Option<Self::State> {
                Some(($($t::initialize(segment, world)?,)*))
            }

            #[inline]
            fn query(index: usize, ($($p,)*): &Self::State) -> Self {
                ($($t::query(index, $p),)*)
            }

            fn dependencies(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $t::dependencies($p));)*
                dependencies
            }
        }
    };
}

crate::recurse!(
    query, query1, Q1, query2, Q2, query3, Q3, query4, Q4, query5, Q5, query6, Q6, query7, Q7,
    query8, Q8, query9, Q9, query10, Q10, query11, Q11, query12, Q12
);
