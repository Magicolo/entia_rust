use crate::{
    defer::{self, Resolve},
    depend::Dependency,
    entities::Entities,
    entity::Entity,
    error::Result,
    family::Family,
    inject::{Adapt, Context, Get, Inject},
    resource::{Read, Write},
    segment::Segments,
};
use entia_core::FullIterator;

pub struct Families<'a>(&'a Entities, &'a Segments);
pub struct State(Read<Entities>, Read<Segments>);

impl<'a> Families<'a> {
    #[inline]
    pub const fn family(&self, entity: Entity) -> Family<'a> {
        Family::new(entity, self.0)
    }

    pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        // Read entities through segments to ensure that partially initialized entities are not observable.
        let entities = self.0;
        self.1
            .iter()
            .flat_map(|segment| unsafe {
                segment.entity_store().get_all::<Entity>(segment.count())
            })
            .filter_map(move |&mut entity| match entities.parent(entity) {
                Some(_) => None,
                None => Some(Family::new(entity, entities)),
            })
    }
}

unsafe impl Inject for Families<'_> {
    type Input = ();
    type State = State;

    fn initialize<A: Adapt<Self::State>>(
        _: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        Ok(State(
            Read::initialize(None, context.map(|State(state, _)| state))?,
            Read::initialize(None, context.map(|State(_, state)| state))?,
        ))
    }

    fn depend(State(entities, segments): &Self::State) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.extend(Read::depend(entities));
        dependencies.extend(Read::depend(segments));
        dependencies
    }
}

impl<'a> Get<'a> for State {
    type Item = Families<'a>;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        Families(self.0.get(), self.1.get())
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

    unsafe impl Inject for Adopt<'_> {
        type Input = ();
        type State = State;

        fn initialize<A: Adapt<Self::State>>(
            _: Self::Input,
            mut context: Context<Self::State, A>,
        ) -> Result<Self::State> {
            let entities = Write::initialize(None, context.map(|state| &mut state.0.as_mut().0))?;
            let inner = Inner(entities);
            let defer = defer::Defer::initialize(inner, context.map(|state| &mut state.0))?;
            Ok(State(defer))
        }

        fn depend(State(state): &Self::State) -> Vec<Dependency> {
            defer::Defer::depend(state)
        }
    }

    unsafe impl Resolve for Inner {
        type Item = Defer;

        fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>) -> Result {
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

        fn depend(&self) -> Vec<Dependency> {
            Write::depend(&self.0)
        }
    }

    impl<'a> Get<'a> for State {
        type Item = Adopt<'a>;

        #[inline]
        unsafe fn get(&'a mut self) -> Self::Item {
            Adopt(self.0.get().0)
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

    unsafe impl Inject for Reject<'_> {
        type Input = ();
        type State = State;

        fn initialize<A: Adapt<Self::State>>(
            _: Self::Input,
            mut context: Context<Self::State, A>,
        ) -> Result<Self::State> {
            let entities = Write::initialize(None, context.map(|state| &mut state.0.as_mut().0))?;
            let inner = Inner(entities);
            let defer = defer::Defer::initialize(inner, context.map(|state| &mut state.0))?;
            Ok(State(defer))
        }

        fn depend(State(state): &Self::State) -> Vec<Dependency> {
            defer::Defer::<Inner>::depend(state)
        }
    }

    unsafe impl Resolve for Inner {
        type Item = Defer;

        fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>) -> Result {
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

        fn depend(&self) -> Vec<Dependency> {
            Write::depend(&self.0)
        }
    }

    impl<'a> Get<'a> for State {
        type Item = Reject<'a>;

        #[inline]
        unsafe fn get(&'a mut self) -> Self::Item {
            Reject(self.0.get().0)
        }
    }
}
