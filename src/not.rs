use std::marker::PhantomData;

use crate::{filter::Filter, segment::Segment, world::World};

pub struct Not<F: Filter>(PhantomData<F>);
pub struct State<T>(PhantomData<T>);

impl<F: Filter> Filter for Not<F> {
    fn filter(segment: &Segment, world: &World) -> bool {
        !F::filter(segment, world)
    }
}
