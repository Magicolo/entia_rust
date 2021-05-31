use crate::depend::Depend;
use crate::segment::*;
use crate::world::*;

pub trait Item {
    type State: for<'a> At<'a> + Depend + Send + 'static;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
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
