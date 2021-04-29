use crate::internal::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct And<Q: Query>(PhantomData<Q>);
pub struct Not<Q: Query>(PhantomData<Q>);

pub trait Query {
    type State;
    fn initialize(segment: &Segment) -> Option<Self::State>;
    fn update(state: &mut Self::State) -> Vec<Dependency>;
    fn resolve(state: &Self::State);
    fn get(index: usize, state: &Self::State) -> Self;
}

impl Query for Entity {
    type State = Arc<SegmentInner>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(segment.inner.clone())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, inner: &Self::State) -> Self {
        inner.entities[index]
    }
}

impl<C: Component> Query for &C {
    type State = (Arc<Vec<Wrap<C>>>, Arc<SegmentInner>);

    fn initialize(segment: &Segment) -> Option<Self::State> {
        let inner = segment.inner.clone();
        let index = inner.indices.get(&TypeId::of::<C>())?;
        let store = inner.stores.get(*index)?;
        let store = store.clone().downcast().ok()?;
        Some((store, inner))
    }

    fn update((_, inner): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(inner.index, TypeId::of::<C>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, (store, _): &Self::State) -> Self {
        unsafe { &*store[index].0.get() }
    }
}

impl<C: Component> Query for &mut C {
    type State = (Arc<Vec<Wrap<C>>>, Arc<SegmentInner>);

    fn initialize(segment: &Segment) -> Option<Self::State> {
        let inner = segment.inner.clone();
        let index = inner.indices.get(&TypeId::of::<C>())?;
        let store = inner.stores.get(*index)?;
        let store = store.clone().downcast().ok()?;
        Some((store, inner))
    }

    fn update((_, inner): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(inner.index, TypeId::of::<C>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, (store, _): &Self::State) -> Self {
        unsafe { &mut *store[index].0.get() }
    }
}

impl<Q: Query> Query for And<Q> {
    type State = ();

    fn initialize(segment: &Segment) -> Option<Self::State> {
        match Q::initialize(segment) {
            Some(_) => Some(()),
            None => None,
        }
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        And(PhantomData)
    }
}

impl<Q: Query> Query for Not<Q> {
    type State = ();

    fn initialize(segment: &Segment) -> Option<Self::State> {
        match Q::initialize(segment) {
            Some(_) => None,
            None => Some(()),
        }
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        Not(PhantomData)
    }
}

impl<Q: Query> Query for Option<Q> {
    type State = Option<Q::State>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(Q::initialize(segment))
    }

    fn update(state: &mut Self::State) -> Vec<Dependency> {
        match state {
            Some(state) => Q::update(state),
            None => Vec::new(),
        }
    }

    fn resolve(state: &Self::State) {
        match state {
            Some(state) => Q::resolve(state),
            None => {}
        }
    }

    #[inline]
    fn get(index: usize, state: &Self::State) -> Self {
        match state {
            Some(state) => Some(Q::get(index, state)),
            None => None,
        }
    }
}

impl Query for () {
    type State = ();

    fn initialize(_: &Segment) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        ()
    }
}

macro_rules! query {
    ($($p:ident, $t:ident),+) => {
        impl<$($t: Query),+> Query for ($($t),+,) {
            type State = ($($t::State),+,);

            fn initialize(segment: &Segment) -> Option<Self::State> {
                Some(($($t::initialize(segment)?),+,))
            }

            fn update(($($p),+,): &mut Self::State) -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $t::update($p)));+;
                dependencies
            }

            fn resolve(($($p),+,): &Self::State) {
                $($t::resolve($p));+;
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
