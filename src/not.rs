use std::marker::PhantomData;

use crate::{
    item::{At, Item},
    modify::Modify,
    segment::Segment,
    system::Dependency,
    world::World,
};

pub struct Not<M: Modify>(PhantomData<M>);
pub struct State<T>(PhantomData<T>);

impl<M: Modify + 'static> Item for Not<M> {
    type State = State<M>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        match M::initialize(segment, world) {
            Some(_) => None,
            None => Some(State(PhantomData)),
        }
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<M: Modify> At<'_> for State<M> {
    type Item = Not<M>;

    #[inline]
    fn at(&self, _: usize) -> Self::Item {
        Not(PhantomData)
    }
}
