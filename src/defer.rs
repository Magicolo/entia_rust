use crate::system::*;
use crate::*;

pub struct Defer {}

impl Defer {
    pub fn create<T>(&self, _entities: &mut [Entity], _template: Template<T>) {}
    pub fn destroy(&self, _entities: &[Entity]) {}
    pub fn add<C: Component>(&self, _entity: Entity, _component: C) {}
    pub fn remove<C: Component>(&self, _entity: Entity) {}
}

impl Inject for Defer {
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
    fn get(_: &Self::State, _: &World) -> Self {
        todo!()
    }
}
