use std::sync::Arc;

use crate::{
    component::Component,
    item::Item,
    segment::Segment,
    world::{Meta, World},
    write::Write,
};

pub trait Initialize: Send + 'static {
    type State: Send;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn metas(world: &mut World) -> Vec<Arc<Meta>>;
    fn set(self, state: &mut Self::State, index: usize);
}

impl<C: Component> Initialize for C {
    type State = <Write<C> as Item>::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        <Write<C> as Item>::initialize(segment, world)
    }

    fn metas(world: &mut World) -> Vec<Arc<Meta>> {
        vec![world.get_or_add_meta::<C>()]
    }

    #[inline]
    fn set(self, state: &mut Self::State, index: usize) {
        // TODO: Ensure that this is called only for initializing a store.
        unsafe { state.store().set(index, self) };
    }
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Initialize,)*> Initialize for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment, _world: &World) -> Option<Self::State> {
                Some(($($t::initialize(_segment, _world)?,)*))
            }

            fn metas(_world: &mut World) -> Vec<Arc<Meta>> {
                let mut _metas = Vec::new();
                $(_metas.append(&mut $t::metas(_world));)*
                _metas
            }

            #[inline]
            fn set(self, ($($p,)*): &mut Self::State, _index: usize) {
                let ($($t,)*) = self;
                $($t.set($p, _index);)*
            }
        }
    };
}

entia_macro::recurse_32!(modify);
