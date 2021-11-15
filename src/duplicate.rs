use std::iter::empty;

use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
    Family, Result,
};

pub struct Duplicate<'a> {
    defer: defer::Defer<'a, Inner>,
    entities: &'a Entities,
    world: &'a World,
}

pub struct State(defer::State<Inner>);

struct Inner {
    entities: write::State<Entities>,
    buffer: Vec<Entity>,
}

struct Defer {}

impl Duplicate<'_> {
    pub fn one(&mut self, entity: Entity, count: usize) -> Option<&[Entity]> {
        let datum = self.entities.get_datum(entity)?;
        Some(&[])
    }

    pub fn family(&mut self, family: Family, count: usize) -> &[Family] {
        &[]
    }
}

impl Inject for Duplicate<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        let inner = Inner {
            entities,
            buffer: Vec::new(),
        };
        let defer = <defer::Defer<Inner> as Inject>::initialize(inner, context)?;
        Ok(State(defer))
    }

    fn resolve(State(state): &mut Self::State, mut context: Context) {
        // Must resolve unconditionally entities and segments *even* if nothing was deferred in the case where creation
        // was completed at run time.
        state.as_mut().resolve(empty(), context.world());
        <defer::Defer<Inner> as Inject>::resolve(state, context.owned());
    }
}

impl<'a> Get<'a> for State {
    type Item = Duplicate<'a>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        let (defer, inner) = self.0.get(world);
        Duplicate {
            defer,
            entities: inner.entities.get(world),
            world,
        }
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) {
        todo!()
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        todo!()
    }
}
