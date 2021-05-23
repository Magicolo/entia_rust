use std::marker::PhantomData;

use crate::{
    inject::Inject, item::Item, modify::Modify, segment::Segment, system::Dependency, world::World,
};

// TODO: Allows to wrap a type 'T' and replace its dependencies with the dependencies of type 'D'.
pub struct Depend<T, D = T>(T, PhantomData<D>);

impl<I: Inject, D> Inject for Depend<I, D> {
    type Input = I::Input;
    type State = I::State;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        I::initialize(input, world)
    }

    fn update(state: &mut Self::State, world: &mut World) {
        I::update(state, world);
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        I::resolve(state, world);
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        todo!();
    }
}

impl<I: Item, D> Item for Depend<I, D> {
    type State = I::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        I::initialize(segment, world)
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        todo!()
    }
}

impl<M: Modify, D> Modify for Depend<M, D> {
    type State = M::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        M::initialize(segment, world)
    }

    fn static_metas(world: &mut World) -> Vec<crate::world::Meta> {
        M::static_metas(world)
    }

    fn dynamic_metas(&self, world: &mut World) -> Vec<crate::world::Meta> {
        self.0.dynamic_metas(world)
    }

    fn validate(&self, state: &Self::State) -> bool {
        self.0.validate(state)
    }

    fn modify(self, state: &Self::State, index: usize) {
        self.0.modify(state, index)
    }

    fn depend(_: &Self::State) -> Vec<Dependency> {
        todo!()
    }
}
