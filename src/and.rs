use std::marker::PhantomData;

use crate::{
    item::{At, Item},
    modify::Modify,
    segment::Segment,
    system::Dependency,
    world::World,
};

pub struct And<M: Modify>(PhantomData<M>);
pub struct State<T>(PhantomData<T>);

impl<M: Modify + 'static> Item for And<M> {
    type State = State<M>;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        match M::initialize(segment, world) {
            Some(_) => Some(State(PhantomData)),
            None => None,
        }
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<M: Modify> At<'_> for State<M> {
    type Item = And<M>;

    #[inline]
    fn at(&self, _: usize) -> Self::Item {
        And(PhantomData)
    }
}
