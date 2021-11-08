use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    family::Family,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
    Result,
};
use std::vec;

pub struct Families<'a>(&'a mut Vec<Defer>, &'a Entities);
pub struct State(Vec<Defer>, write::State<Entities>);
enum Defer {
    AdoptAt(Entity, Entity, usize),
    AdoptFirst(Entity, Entity),
    AdoptLast(Entity, Entity),
    AdoptBefore(Entity, Entity),
    AdoptAfter(Entity, Entity),
    Reject(Entity),
    RejectAt(Entity, usize),
    RejectFirst(Entity),
    RejectLast(Entity),
    RejectAll(Entity),
}

impl<'a> Families<'a> {
    #[inline]
    pub const fn family(&self, entity: Entity) -> Family<'a> {
        Family::new(entity, self.1)
    }

    #[inline]
    pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let entities = self.1;
        entities
            .roots()
            .map(move |entity| Family::new(entity, entities))
    }

    #[inline]
    pub fn adopt_at(&mut self, parent: Entity, child: Entity, index: usize) {
        self.0.push(Defer::AdoptAt(parent, child, index));
    }

    #[inline]
    pub fn adopt_first(&mut self, parent: Entity, child: Entity) {
        self.0.push(Defer::AdoptFirst(parent, child));
    }

    #[inline]
    pub fn adopt_last(&mut self, parent: Entity, child: Entity) {
        self.0.push(Defer::AdoptLast(parent, child));
    }

    #[inline]
    pub fn adopt_before(&mut self, sibling: Entity, child: Entity) {
        self.0.push(Defer::AdoptBefore(sibling, child));
    }

    #[inline]
    pub fn adopt_after(&mut self, sibling: Entity, child: Entity) {
        self.0.push(Defer::AdoptAfter(sibling, child));
    }

    #[inline]
    pub fn reject_first(&mut self, parent: Entity) {
        self.0.push(Defer::RejectFirst(parent));
    }

    #[inline]
    pub fn reject_last(&mut self, parent: Entity) {
        self.0.push(Defer::RejectLast(parent));
    }

    #[inline]
    pub fn reject_at(&mut self, parent: Entity, index: usize) {
        self.0.push(Defer::RejectAt(parent, index));
    }

    #[inline]
    pub fn reject(&mut self, child: Entity) {
        self.0.push(Defer::Reject(child));
    }

    #[inline]
    pub fn reject_all(&mut self, parent: Entity) {
        self.0.push(Defer::RejectAll(parent));
    }
}

impl Inject for Families<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, context: Context) -> Result<Self::State> {
        Ok(State(
            Vec::new(),
            <Write<Entities> as Inject>::initialize(None, context)?,
        ))
    }

    fn resolve(state: &mut Self::State, _: Context) {
        let entities = state.1.as_mut();
        for defer in state.0.drain(..) {
            match defer {
                Defer::AdoptAt(parent, child, index) => {
                    entities.adopt_at(parent, child, index);
                }
                Defer::AdoptFirst(parent, child) => {
                    entities.adopt_first(parent, child);
                }
                Defer::AdoptLast(parent, child) => {
                    entities.adopt_last(parent, child);
                }
                Defer::AdoptBefore(sibling, child) => {
                    entities.adopt_before(sibling, child);
                }
                Defer::AdoptAfter(sibling, child) => {
                    entities.adopt_after(sibling, child);
                }
                Defer::Reject(child) => {
                    entities.reject(child);
                }
                Defer::RejectAt(parent, index) => {
                    entities.reject_at(parent, index);
                }
                Defer::RejectFirst(parent) => {
                    entities.reject_first(parent);
                }
                Defer::RejectLast(parent) => {
                    entities.reject_last(parent);
                }
                Defer::RejectAll(parent) => {
                    entities.reject_all(parent);
                }
            }
        }
    }
}

impl<'a> Get<'a> for State {
    type Item = Families<'a>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Families(&mut self.0, self.1.get(world))
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![
            Dependency::defer::<Entities>(),
            Dependency::read::<Entities>(),
        ]
    }
}
