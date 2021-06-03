use std::marker::PhantomData;

use crate::{segment::Segment, world::World};

pub trait Filter: Send + 'static {
    fn filter(segment: &Segment, world: &World) -> bool;
}

pub struct Not<F: Filter>(PhantomData<F>);

impl<F: Filter> Filter for Not<F> {
    fn filter(segment: &Segment, world: &World) -> bool {
        !F::filter(segment, world)
    }
}

macro_rules! filter {
    ($($t:ident, $p:ident),*) => {
        impl<$($t: Filter,)*> Filter for ($($t,)*) {
            fn filter(_segment: &Segment, _world: &World) -> bool {
                $($t::filter(_segment, _world) &&)* true
            }
        }
    };
}

entia_macro::recurse_32!(filter);
