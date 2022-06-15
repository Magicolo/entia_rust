use std::marker::PhantomData;

use crate::{recurse, segment::Segment, world::World};

pub trait Filter {
    fn filter(segment: &Segment, world: &World) -> bool;
}

#[derive(Copy, Clone, Debug)]
pub struct Has<T>(PhantomData<T>);
#[derive(Copy, Clone, Debug)]
pub struct Not<F>(PhantomData<F>);

impl<T: Send + Sync + 'static> Filter for Has<T> {
    fn filter(segment: &Segment, world: &World) -> bool {
        if let Ok(meta) = world.get_meta::<T>() {
            segment.component_types().contains(&meta.identifier())
        } else {
            false
        }
    }
}

impl<F: Filter> Filter for Not<F> {
    fn filter(segment: &Segment, world: &World) -> bool {
        !F::filter(segment, world)
    }
}

impl<T> Filter for PhantomData<T> {
    fn filter(segment: &Segment, world: &World) -> bool {
        <() as Filter>::filter(segment, world)
    }
}

macro_rules! filter {
        ($($p:ident, $t:ident),*) => {
            impl<$($t: Filter,)*> Filter for ($($t,)*) {
                fn filter(_segment: &Segment, _world: &World) -> bool {
                    $($t::filter(_segment, _world) &&)* true
                }
            }
        };
    }

recurse!(filter);
