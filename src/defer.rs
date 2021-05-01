use crate::system::*;
use crate::*;

pub struct Defer<'a> {
    world: &'a World,
}

impl Defer<'_> {
    pub fn create<T>(&self, entities: &mut [Entity], template: Template<T>) {
        todo!()
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

impl Inject for Defer<'_> {
    type State = ();

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State, _: &mut World) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State, _: &mut World) {
        todo!()
    }

    #[inline]
    fn get(_: &Self::State, world: &World) -> Self {
        todo!()
        // Self { world }
    }
}
