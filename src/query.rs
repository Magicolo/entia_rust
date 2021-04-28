use crate::component::Segment;
use crate::dependency::Dependency;
use crate::*;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub trait Query<'a> {
    type State: 'a;

    fn dependencies() -> Vec<Dependency>;
    fn query(segment: &Segment) -> Option<Self::State>;
    fn get(index: usize, state: &'a Self::State, segment: &'a Segment) -> Self;
}

// Entities can be queried
impl Query<'_> for Entity {
    type State = ();

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>()]
    }

    fn query(_: &Segment) -> Option<()> {
        Some(())
    }

    #[inline(always)]
    fn get(index: usize, _: &Self::State, segment: &Segment) -> Self {
        unsafe { segment.get().entities[index] }
    }
}

// Components can be queried
impl<'a, C: Component + 'static> Query<'a> for &'a C {
    type State = Rc<UnsafeCell<Vec<C>>>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>(), Dependency::read::<C>()]
    }

    fn query(segment: &Segment) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(index: usize, state: &Self::State, _: &Segment) -> Self {
        let store = unsafe { &*state.get() };
        &store[index]
    }
}

impl<'a, C: Component + 'static> Query<'a> for &'a mut C {
    type State = Rc<UnsafeCell<Vec<C>>>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>(), Dependency::write::<C>()]
    }

    fn query(segment: &Segment) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(index: usize, state: &Self::State, _: &Segment) -> Self {
        let store = unsafe { &mut *state.get() };
        &mut store[index]
    }
}

// Support for optional queries
impl<'a, Q: Query<'a>> Query<'a> for Option<Q> {
    type State = Option<Q::State>;

    fn dependencies() -> Vec<Dependency> {
        Q::dependencies()
    }

    fn query(segment: &Segment) -> Option<Self::State> {
        Some(Q::query(segment))
    }

    #[inline(always)]
    fn get(index: usize, state: &'a Self::State, segment: &'a Segment) -> Self {
        state.as_ref().map(|state| Q::get(index, state, segment))
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

            fn dependencies() -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $q::dependencies());)+
                dependencies
            }

            fn query(segment: &Segment) -> Option<Self::State> {
                match ($($q::query(segment)),+) {
                    ($(Some($s)),+) => Some(($($s),+)),
                    _ => None,
                }
            }

            #[inline(always)]
            fn get(index: usize, ($($s),+): &'a Self::State, segment: &'a Segment) -> Self {
                ($($q::get(index, $s, segment)),+)
            }
        }
    };
}

tuples!(
    query, Q0, state0, Q1, state1, Q2, state2, Q3, state3, Q4, state4, Q5, state5, Q6, state6, Q7,
    state7, Q8, state8, Q9, state9
);
