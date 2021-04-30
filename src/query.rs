use crate::system::*;
use crate::world::*;
use std::marker::PhantomData;

pub struct And<Q: Query>(PhantomData<Q>);
pub struct Not<Q: Query>(PhantomData<Q>);

pub trait Query {
    type State;
    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)>;
    fn get(index: usize, state: &Self::State) -> Self;
}

impl Query for () {
    type State = ();

    fn initialize(_: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        Some(((), Vec::new()))
    }

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        ()
    }
}

macro_rules! query {
    ($($p:ident, $t:ident),+) => {
        impl<$($t: Query),+> Query for ($($t),+,) {
            type State = ($($t::State),+,);

            fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
                let mut dependencies = Vec::new();
                $(let mut $p = $t::initialize(segment)?);+;
                $(dependencies.append(&mut $p.1));+;
                Some((($($p.0),+,), dependencies))
            }

            #[inline]
            fn get(index: usize, ($($p),+,): &Self::State) -> Self {
                ($($t::get(index, $p)),+,)
            }
        }
    };
}

crate::recurse!(
    query, query1, Q1, query2, Q2, query3, Q3, query4, Q4, query5, Q5, query6, Q6, query7, Q7,
    query8, Q8, query9, Q9, query10, Q10, query11, Q11, query12, Q12
);

impl<Q: Query> Query for Option<Q> {
    type State = Option<Q::State>;

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        Some(match Q::initialize(segment) {
            Some(pair) => (Some(pair.0), pair.1),
            None => (None, Vec::new()),
        })
    }

    #[inline]
    fn get(index: usize, state: &Self::State) -> Self {
        match state {
            Some(state) => Some(Q::get(index, state)),
            None => None,
        }
    }
}

impl<Q: Query> Query for And<Q> {
    type State = ();

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        match Q::initialize(segment) {
            Some(_) => Some(((), Vec::new())),
            None => None,
        }
    }

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        And(PhantomData)
    }
}

impl<Q: Query> Query for Not<Q> {
    type State = ();

    fn initialize(segment: &Segment) -> Option<(Self::State, Vec<Dependency>)> {
        match Q::initialize(segment) {
            Some(_) => None,
            None => Some(((), Vec::new())),
        }
    }

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        Not(PhantomData)
    }
}
