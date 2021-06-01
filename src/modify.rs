use std::sync::Arc;

use crate::{
    component::Component,
    segment::{Segment, Store},
    world::{Meta, World},
};

pub trait Modify: Send + 'static {
    type State: Send;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn static_metas(world: &mut World) -> Vec<Arc<Meta>>;
    fn dynamic_metas(&self, world: &mut World) -> Vec<Arc<Meta>>;
    fn validate(&self, state: &Self::State) -> bool;
    fn modify(self, state: &mut Self::State, index: usize);
}

pub trait Homogeneous {}

impl<C: Component> Modify for C {
    type State = (Store, usize);

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        let meta = world.get_meta::<C>()?;
        let store = unsafe { segment.store(&meta)?.clone() };
        Some((store, segment.index))
    }

    fn static_metas(world: &mut World) -> Vec<Arc<Meta>> {
        vec![world.get_or_add_meta::<C>()]
    }

    fn dynamic_metas(&self, world: &mut World) -> Vec<Arc<Meta>> {
        Self::static_metas(world)
    }

    #[inline]
    fn validate(&self, _: &Self::State) -> bool {
        true
    }

    #[inline]
    fn modify(self, (store, _): &mut Self::State, index: usize) {
        unsafe { store.set(index, self) };
    }
}

impl<C: Component> Homogeneous for C {}

impl<M: Modify> Modify for Option<M> {
    type State = Option<M::State>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        Some(M::initialize(segment, world))
    }

    fn static_metas(world: &mut World) -> Vec<Arc<Meta>> {
        M::static_metas(world)
    }

    fn dynamic_metas(&self, world: &mut World) -> Vec<Arc<Meta>> {
        match self {
            Some(modify) => modify.dynamic_metas(world),
            None => Vec::new(),
        }
    }

    #[inline]
    fn validate(&self, state: &Self::State) -> bool {
        match (self, state) {
            (Some(modify), Some(state)) => modify.validate(state),
            (Some(_), None) => false,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    }

    #[inline]
    fn modify(self, state: &mut Self::State, index: usize) {
        match (self, state) {
            (Some(modify), Some(state)) => modify.modify(state, index),
            _ => {}
        }
    }
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Modify,)*> Modify for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment, _world: &World) -> Option<Self::State> {
                Some(($($t::initialize(_segment, _world)?,)*))
            }

            fn static_metas(_world: &mut World) -> Vec<Arc<Meta>> {
                let mut _metas = Vec::new();
                $(_metas.append(&mut $t::static_metas(_world));)*
                _metas
            }

            fn dynamic_metas(&self, _world: &mut World) -> Vec<Arc<Meta>> {
                let ($($p,)*) = self;
                let mut _metas = Vec::new();
                $(_metas.append(&mut $p.dynamic_metas(_world));)*
                _metas
            }

            #[inline]
            fn validate(&self, ($($p,)*): &Self::State) -> bool {
                let ($($t,)*) = self;
                $($t.validate($p) && )* true
            }

            #[inline]
            fn modify(self, ($($p,)*): &mut Self::State, _index: usize) {
                let ($($t,)*) = self;
                $($t.modify($p, _index);)*
            }
        }

        impl<$($t: Homogeneous,)*> Homogeneous for ($($t,)*) {}
    };
}

entia_macro::recurse_32!(modify);
