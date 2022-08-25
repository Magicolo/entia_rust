use std::{any::TypeId, marker::PhantomData};

use crate::{component::Component, segment::Segment, tuples};

pub trait Filter {
    fn filter(segment: &Segment) -> bool;
}

#[derive(Copy, Clone, Debug)]
pub struct Has<T>(PhantomData<T>);
#[derive(Copy, Clone, Debug)]
pub struct Not<F>(PhantomData<F>);

impl<T: Component> Filter for Has<T> {
    fn filter(segment: &Segment) -> bool {
        segment.types().contains(&TypeId::of::<T>())
    }
}

impl<F: Filter> Filter for Not<F> {
    fn filter(segment: &Segment) -> bool {
        !F::filter(segment)
    }
}

impl<T> Filter for PhantomData<T> {
    fn filter(segment: &Segment) -> bool {
        <() as Filter>::filter(segment)
    }
}

macro_rules! filter {
        ($($p:ident, $t:ident),*) => {
            impl<$($t: Filter,)*> Filter for ($($t,)*) {
                fn filter(_segment: &Segment) -> bool {
                    $($t::filter(_segment) &&)* true
                }
            }
        };
    }

tuples!(filter);
