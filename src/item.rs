use crate::segment::*;
use crate::system::*;
use crate::world::*;

pub trait Item {
    type State: for<'a> At<'a> + Send + 'static;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

pub trait At<'a> {
    type Item;
    fn at(&'a self, index: usize) -> Self::Item;
}

impl<I: Item> Item for Option<I> {
    type State = Option<I::State>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        Some(I::initialize(segment, world))
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

macro_rules! item {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Item,)*> Item for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment, _world: &World) -> Option<Self::State> {
                Some(($($t::initialize(_segment, _world)?,)*))
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

entia_macro::recurse_32!(item);
