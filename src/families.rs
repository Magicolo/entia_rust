use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    family::Family,
    inject::{Get, Inject, InjectContext},
    world::World,
    write::{self, Write},
};
use std::vec;

pub struct Families<'a>(&'a mut Vec<Defer>, &'a Entities);
pub struct State(Vec<Defer>, write::State<Entities>);
enum Defer {
    AdoptFirst(Entity, Entity),
    AdoptLast(Entity, Entity),
    Reject(Entity),
    RejectAll(Entity),
}

impl<'a> Families<'a> {
    #[inline]
    pub const fn family(&self, entity: Entity) -> Family<'a> {
        Family::new(entity, self.1)
    }

    pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let entities = self.1;
        entities
            .roots()
            .map(move |entity| Family::new(entity, entities))
    }

    pub fn adopt_first(&mut self, parent: Entity, child: Entity) {
        self.0.push(Defer::AdoptFirst(parent, child));
    }

    pub fn adopt_last(&mut self, parent: Entity, child: Entity) {
        self.0.push(Defer::AdoptLast(parent, child));
    }

    pub fn reject_first(&mut self, parent: Entity) {
        if let Some(child) = self.1.children(parent).next() {
            self.reject(child);
        }
    }

    pub fn reject_last(&mut self, parent: Entity) {
        if let Some(child) = self.1.children(parent).next_back() {
            self.reject(child);
        }
    }

    pub fn reject_at(&mut self, parent: Entity, index: usize) {
        if let Some(child) = self.1.children(parent).nth(index) {
            self.reject(child);
        }
    }

    pub fn reject(&mut self, child: Entity) {
        self.0.push(Defer::Reject(child));
    }

    pub fn reject_all(&mut self, parent: Entity) {
        Defer::RejectAll(parent);
    }
}

unsafe impl Inject for Families<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, context: InjectContext) -> Option<Self::State> {
        Some(State(
            Vec::new(),
            <Write<Entities> as Inject>::initialize(None, context)?,
        ))
    }

    fn resolve(state: &mut Self::State, _: InjectContext) {
        let entities = state.1.as_mut();
        for defer in state.0.drain(..) {
            match defer {
                Defer::AdoptFirst(parent, child) => {
                    entities.adopt_first(parent, child);
                }
                Defer::AdoptLast(parent, child) => {
                    entities.adopt_last(parent, child);
                }
                Defer::Reject(child) => {
                    entities.reject(child);
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
        // TODO: As it stands, 'Families' could be injected twice in a system. While it would respect Rust's invariants, it might
        // have an unintuitive resolution.
        vec![
            Dependency::defer::<Entities>(),
            Dependency::read::<Entities>(),
        ]
    }
}
