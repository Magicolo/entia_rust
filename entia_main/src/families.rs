use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    family::Family,
    inject::{Context, Get, Inject},
    resource::{Read, Write},
    world::World,
};
use entia_core::FullIterator;

pub struct Families<'a>(&'a Entities);
pub struct State(Read<Entities>);

impl<'a> Families<'a> {
    #[inline]
    pub const fn family(&self, entity: Entity) -> Family<'a> {
        Family::new(entity, self.0)
    }

    // TODO: This read over all 'Datum' allows to observe entities that are not fully initialized from 'Create' in a
    // non-deterministic way and possibly data-racy way.
    // - Can be fixed by adding a validation step: 'datum.store < world.segments[datum.segment].count'
    #[inline]
    pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let entities = self.0;
        entities
            .roots()
            .map(move |entity| Family::new(entity, entities))
    }
}

impl Inject for Families<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        Ok(State(Read::<Entities>::initialize(None, context.owned())?))
    }
}

impl<'a> Get<'a> for State {
    type Item = Families<'a>;

    #[inline]
    unsafe fn get(&'a mut self, world: &'a World) -> Self::Item {
        Families(self.0.get(world))
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.0.depend(world)
    }
}

pub mod adopt {
    use super::*;

    pub struct Adopt<'a>(defer::Defer<'a, Inner>);
    pub struct State(defer::State<Inner>);
    struct Inner(Write<Entities>);

    enum Defer {
        At(Entity, Entity, usize),
        First(Entity, Entity),
        Last(Entity, Entity),
        Before(Entity, Entity),
        After(Entity, Entity),
    }

    impl Adopt<'_> {
        #[inline]
        pub fn at(&mut self, parent: impl Into<Entity>, child: impl Into<Entity>, index: usize) {
            self.0.one(Defer::At(parent.into(), child.into(), index));
        }

        #[inline]
        pub fn first(&mut self, parent: impl Into<Entity>, child: impl Into<Entity>) {
            self.0.one(Defer::First(parent.into(), child.into()));
        }

        #[inline]
        pub fn last(&mut self, parent: impl Into<Entity>, child: impl Into<Entity>) {
            self.0.one(Defer::Last(parent.into(), child.into()));
        }

        #[inline]
        pub fn before(&mut self, sibling: impl Into<Entity>, child: impl Into<Entity>) {
            self.0.one(Defer::Before(sibling.into(), child.into()));
        }

        #[inline]
        pub fn after(&mut self, sibling: impl Into<Entity>, child: impl Into<Entity>) {
            self.0.one(Defer::After(sibling.into(), child.into()));
        }
    }

    impl Resolve for Inner {
        type Item = Defer;

        fn resolve(
            &mut self,
            items: impl FullIterator<Item = Self::Item>,
            _: &mut World,
        ) -> Result {
            for defer in items {
                match defer {
                    Defer::At(parent, child, index) => {
                        self.0.adopt_at(parent, child, index);
                    }
                    Defer::First(parent, child) => {
                        self.0.adopt_first(parent, child);
                    }
                    Defer::Last(parent, child) => {
                        self.0.adopt_last(parent, child);
                    }
                    Defer::Before(sibling, child) => {
                        self.0.adopt_before(sibling, child);
                    }
                    Defer::After(sibling, child) => {
                        self.0.adopt_after(sibling, child);
                    }
                }
            }
            Ok(())
        }
    }

    impl<'a> Get<'a> for State {
        type Item = Adopt<'a>;

        #[inline]
        unsafe fn get(&'a mut self, world: &'a World) -> Self::Item {
            Adopt(self.0.get(world).0)
        }
    }

    unsafe impl Depend for State {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.0.depend(world);
            dependencies.push(Dependency::defer::<Entity>());
            dependencies
        }
    }
}

pub mod reject {
    use super::*;

    pub struct Reject<'a>(defer::Defer<'a, Inner>);
    pub struct State(defer::State<Inner>);
    struct Inner(Write<Entities>);

    enum Defer {
        One(Entity),
        At(Entity, usize),
        First(Entity),
        Last(Entity),
        All(Entity),
    }

    impl Reject<'_> {
        #[inline]
        pub fn one(&mut self, child: impl Into<Entity>) {
            self.0.one(Defer::One(child.into()));
        }

        #[inline]
        pub fn first(&mut self, parent: impl Into<Entity>) {
            self.0.one(Defer::First(parent.into()));
        }

        #[inline]
        pub fn last(&mut self, parent: impl Into<Entity>) {
            self.0.one(Defer::Last(parent.into()));
        }

        #[inline]
        pub fn at(&mut self, parent: impl Into<Entity>, index: usize) {
            self.0.one(Defer::At(parent.into(), index));
        }

        #[inline]
        pub fn all(&mut self, parent: impl Into<Entity>) {
            self.0.one(Defer::All(parent.into()));
        }
    }

    impl Resolve for Inner {
        type Item = Defer;

        fn resolve(
            &mut self,
            items: impl FullIterator<Item = Self::Item>,
            _: &mut World,
        ) -> Result {
            for defer in items {
                match defer {
                    Defer::One(child) => {
                        self.0.reject(child);
                    }
                    Defer::At(parent, index) => {
                        self.0.reject_at(parent, index);
                    }
                    Defer::First(parent) => {
                        self.0.reject_first(parent);
                    }
                    Defer::Last(parent) => {
                        self.0.reject_last(parent);
                    }
                    Defer::All(parent) => {
                        self.0.reject_all(parent);
                    }
                }
            }
            Ok(())
        }
    }

    impl<'a> Get<'a> for State {
        type Item = Reject<'a>;

        #[inline]
        unsafe fn get(&'a mut self, world: &'a World) -> Self::Item {
            Reject(self.0.get(world).0)
        }
    }

    unsafe impl Depend for State {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.0.depend(world);
            dependencies.push(Dependency::defer::<Entity>());
            dependencies
        }
    }
}
