use crate::system::*;
use crate::world::*;
use crate::*;

pub struct Defer<'a>(&'a World);

impl Defer<'_> {
    pub fn create<T, const N: usize>(&self, template: Template<T>) -> [Entity; N] {
        todo!();
        // use std::mem::MaybeUninit;
        // let entities = MaybeUninit::uninit_array();
        // unsafe { MaybeUninit::array_assume_init(entities) }
    }
    pub fn destroy(&self, entities: &[Entity]) {
        todo!()
    }
    pub fn add<C: Component>(&self, entity: Entity, _component: C) {
        todo!()
    }
    pub fn remove<C: Component>(&self, entity: Entity) {
        todo!()
    }
}

impl<'a> Inject<'a> for Defer<'a> {
    type State = &'a World;

    fn initialize(world: &'a World) -> Option<Self::State> {
        Some(world)
    }

    fn inject(state: &Self::State) -> Self {
        Defer(state)
    }

    fn resolve(_: &mut Self::State) {
        todo!()
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}
