use crate::initialize::*;
use crate::system::*;
use crate::world::*;
use crate::*;

pub struct Defer<'a>(&'a World);

impl Defer<'_> {
    pub fn create<T, const N: usize>(&self, _: impl Initialize) -> [Entity; N] {
        todo!();
        // use std::mem::MaybeUninit;
        // let entities = MaybeUninit::uninit_array();
        // unsafe { MaybeUninit::array_assume_init(entities) }
    }

    pub fn destroy(&self, _: &[Entity]) {
        todo!()
    }

    pub fn add<C: Component>(&self, _: Entity, _component: C) {
        todo!()
    }

    pub fn remove<C: Component>(&self, _: Entity) {
        todo!()
    }
}

impl Inject for Defer<'_> {
    type Input = ();
    type State = (); //&'a World;

    fn initialize(_: Self::Input, _: &mut World) -> Option<Self::State> {
        todo!()
        // Some(world)
    }

    // fn inject(state: &Self::State) -> Self {
    //     Defer(state)
    // }

    fn resolve(_: &mut Self::State, _: &mut World) {
        todo!()
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}
