use std::{any::TypeId, sync::Arc};

use crate::{
    component::Component,
    segment::Segment,
    system::Dependency,
    world::{Meta, Store, World},
};

pub trait Modify {
    type State;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn metas(&self, world: &mut World) -> Vec<Meta>;
    fn validate(&self, state: &Self::State) -> bool;
    fn modify(self, state: &Self::State, index: usize);
    fn depend(state: &Self::State) -> Vec<Dependency>;
}

impl<C: Component> Modify for C {
    type State = (Arc<Store<C>>, usize);

    fn initialize(segment: &Segment, _: &World) -> Option<Self::State> {
        Some((segment.static_store()?, segment.index))
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        vec![world.get_or_add_meta::<C>()]
    }

    #[inline]
    fn validate(&self, _: &Self::State) -> bool {
        true
    }

    #[inline]
    fn modify(self, (store, _): &Self::State, index: usize) {
        *unsafe { store.at(index) } = self;
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<C>())]
    }
}

impl<M: Modify> Modify for Option<M> {
    type State = M::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        M::initialize(segment, world)
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        match self {
            Some(modify) => modify.metas(world),
            None => Vec::new(),
        }
    }

    #[inline]
    fn validate(&self, state: &Self::State) -> bool {
        match self {
            Some(modify) => modify.validate(state),
            _ => false,
        }
    }

    #[inline]
    fn modify(self, state: &Self::State, index: usize) {
        match self {
            Some(modify) => modify.modify(state, index),
            _ => {}
        }
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        M::depend(state)
    }
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Modify,)*> Modify for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment, _world: &World) -> Option<Self::State> {
                Some(($($t::initialize(_segment, _world)?,)*))
            }

            fn metas(&self, _world: &mut World) -> Vec<Meta> {
                let ($($p,)*) = self;
                let mut _metas = Vec::new();
                $(_metas.append(&mut $p.metas(_world));)*
                _metas
            }

            #[inline]
            fn validate(&self, ($($p,)*): &Self::State) -> bool {
                let ($($t,)*) = self;
                $($t.validate($p) && )* true
            }

            #[inline]
            fn modify(self, ($($p,)*): &Self::State, _index: usize) {
                let ($($t,)*) = self;
                $($t.modify($p, _index);)*
            }

            fn depend(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::depend($p));)*
                _dependencies
            }
        }
    };
}

entia_macro::recurse_32!(modify);
