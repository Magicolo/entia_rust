use crate::component::Segment;
use crate::*;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub trait Query<'a> {
    type State: 'a;

    fn state(segment: &Segment) -> Option<Self::State>;
    fn query(index: usize, state: &'a Self::State, segment: &'a Segment) -> Self;
}

// Entities can be queried
impl Query<'_> for Entity {
    type State = ();

    fn state(_: &Segment) -> Option<()> {
        Some(())
    }

    #[inline(always)]
    fn query(index: usize, _: &Self::State, segment: &Segment) -> Self {
        segment.entities[index]
    }
}

// Components can be queried
impl<'a, C: Component + 'static> Query<'a> for &'a C {
    type State = Rc<UnsafeCell<Vec<C>>>;

    fn state(segment: &Segment) -> Option<Self::State> {
        segment.get_store()
    }

    #[inline(always)]
    fn query(index: usize, state: &Self::State, _: &Segment) -> Self {
        let store = unsafe { &*state.get() };
        &store[index]
    }
}

impl<'a, C: Component + 'static> Query<'a> for &'a mut C {
    type State = Rc<UnsafeCell<Vec<C>>>;

    fn state(segment: &Segment) -> Option<Self::State> {
        segment.get_store()
    }

    #[inline(always)]
    fn query(index: usize, state: &Self::State, _: &Segment) -> Self {
        let store = unsafe { &mut *state.get() };
        &mut store[index]
    }
}

// Support for optional queries
impl<'a, Q: Query<'a>> Query<'a> for Option<Q> {
    type State = Option<Q::State>;

    fn state(segment: &Segment) -> Option<Self::State> {
        Some(Q::state(segment))
    }

    #[inline(always)]
    fn query(index: usize, state: &'a Self::State, segment: &'a Segment) -> Self {
        state.as_ref().map(|state| Q::query(index, state, segment))
    }
}

macro_rules! tuples {
    ($m:ident, $p:ident, $s:ident) => {};
    ($m:ident, $p:ident, $s:ident, $($ps:ident, $ss:ident),+) => {
        $m!($p, $s, $($ps, $ss),+);
        tuples!($m, $($ps, $ss),+);
    };
}

macro_rules! query {
    ($($q:ident, $s:ident),+) => {
        impl<'a, $($q: Query<'a>),+ > Query<'a> for ($($q),+) {
            type State = ($($q::State),+);

            fn state(segment: &Segment) -> Option<Self::State> {
                match ($($q::state(segment)),+) {
                    ($(Some($s)),+) => Some(($($s),+)),
                    _ => None,
                }
            }

            #[inline(always)]
            fn query(index: usize, ($($s),+): &'a Self::State, segment: &'a Segment) -> Self {
                ($($q::query(index, $s, segment)),+)
            }
        }
    };
}

tuples!(
    query, Q0, state0, Q1, state1, Q2, state2, Q3, state3, Q4, state4, Q5, state5, Q6, state6, Q7,
    state7, Q8, state8, Q9, state9
);
