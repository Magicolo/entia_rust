use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    family::Family,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
};

pub struct Families<'a>(defer::Defer<'a, Inner>, &'a Entities);
pub struct State(defer::State<Inner>);

struct Inner(write::State<Entities>);

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

    // TODO: This read over all 'Datum' allows to observe entities that are not fully initialized from 'Create' in a
    // non-deterministic way and possibly data-racy way.
    // - Can be fixed by adding a validation step: 'datum.store < world.segments[datum.segment].count'
    // #[inline]
    // pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
    //     let entities = self.1;
    //     entities
    //         .roots()
    //         .map(move |entity| Family::new(entity, entities))
    // }

    #[inline]
    pub fn adopt_at(&mut self, parent: Entity, child: Entity, index: usize) {
        self.0.defer(Defer::AdoptAt(parent, child, index));
    }

    #[inline]
    pub fn adopt_first(&mut self, parent: Entity, child: Entity) {
        self.0.defer(Defer::AdoptFirst(parent, child));
    }

    #[inline]
    pub fn adopt_last(&mut self, parent: Entity, child: Entity) {
        self.0.defer(Defer::AdoptLast(parent, child));
    }

    #[inline]
    pub fn adopt_before(&mut self, sibling: Entity, child: Entity) {
        self.0.defer(Defer::AdoptBefore(sibling, child));
    }

    #[inline]
    pub fn adopt_after(&mut self, sibling: Entity, child: Entity) {
        self.0.defer(Defer::AdoptAfter(sibling, child));
    }

    #[inline]
    pub fn reject_first(&mut self, parent: Entity) {
        self.0.defer(Defer::RejectFirst(parent));
    }

    #[inline]
    pub fn reject_last(&mut self, parent: Entity) {
        self.0.defer(Defer::RejectLast(parent));
    }

    #[inline]
    pub fn reject_at(&mut self, parent: Entity, index: usize) {
        self.0.defer(Defer::RejectAt(parent, index));
    }

    #[inline]
    pub fn reject(&mut self, child: Entity) {
        self.0.defer(Defer::Reject(child));
    }

    #[inline]
    pub fn reject_all(&mut self, parent: Entity) {
        self.0.defer(Defer::RejectAll(parent));
    }
}

impl Inject for Families<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let inner = Inner(<Write<Entities> as Inject>::initialize(
            None,
            context.owned(),
        )?);
        let defer = <defer::Defer<Inner> as Inject>::initialize(inner, context)?;
        Ok(State(defer))
    }

    fn resolve(State(state): &mut Self::State, context: Context) -> Result {
        <defer::Defer<Inner> as Inject>::resolve(state, context)
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, _: &mut World) -> Result {
        let entities = self.0.as_mut();
        for defer in items {
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
        Ok(())
    }
}

impl<'a> Get<'a> for State {
    type Item = Families<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        let (defer, inner) = self.0.get(world);
        Families(defer, inner.0.get(world))
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        dependencies.push(Dependency::defer::<Entities>());
        dependencies.push(Dependency::read::<Entities>());
        dependencies
    }
}
