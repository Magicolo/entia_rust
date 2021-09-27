use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject, InjectContext},
    item::{At, Item},
    query::{self, Query},
    world::World,
};
use std::{any::TypeId, marker::PhantomData};

pub struct Destroy<'a, I: Item + 'static = (), F: Filter = ()> {
    defer: &'a mut Vec<Defer<I, F>>,
    query: Query<'a, I, (Entity, F)>,
}

pub struct State<I: Item + 'static, F: Filter> {
    defer: Vec<Defer<I, F>>,
    query: query::State<I, (Entity, F)>,
}

enum Defer<I: Item, F: Filter> {
    One(Entity),
    All(PhantomData<(I, F)>),
}

impl<I: Item, F: Filter> Destroy<'_, I, F> {
    #[inline]
    pub fn one(&mut self, entity: Entity) {
        if let Some(_) = self.query.get(entity) {
            self.defer.push(Defer::One(entity));
        }
    }

    #[inline]
    pub fn one_with(
        &mut self,
        entity: Entity,
        filter: impl FnOnce(<I::State as At<'_>>::Item) -> bool,
    ) {
        if let Some(item) = self.query.get(entity) {
            if filter(item) {
                self.defer.push(Defer::One(entity));
            }
        }
    }

    #[inline]
    pub fn all(&mut self) {
        self.defer.push(Defer::All(PhantomData));
    }

    #[inline]
    pub fn all_with(&mut self, filter: impl FnMut(Entity, I) -> bool) {
        todo!()
    }
}

unsafe impl<I: Item, F: Filter> Inject for Destroy<'_, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, context: InjectContext) -> Option<Self::State> {
        let query = <Query<I, (Entity, F)> as Inject>::initialize((), context)?;
        Some(State {
            defer: Vec::new(),
            query,
        })
    }

    fn update(state: &mut Self::State, context: InjectContext) {
        <Query<I, (Entity, F)> as Inject>::update(&mut state.query, context);
    }

    fn resolve(state: &mut Self::State, mut context: InjectContext) {
        <Query<I, (Entity, F)> as Inject>::resolve(&mut state.query, context.owned());

        let entities = state.query.entities.as_mut();
        let world = context.world();
        let query = state.query.inner.as_mut();
        for defer in state.defer.drain(..) {
            match defer {
                Defer::One(entity) => {
                    if let Some(datum) = entities.get_datum_mut(entity) {
                        let index = datum.index() as usize;
                        let segment = datum.segment() as usize;
                        if query.segments[segment].is_some() {
                            entities.release(&[entity]);
                            let segment = &mut world.segments[segment];
                            if segment.remove_at(index) {
                                // SAFETY: When it exists, the entity store is always the first. This segment must have
                                // an entity store since the destroyed entity was in it.
                                let moved = *unsafe { segment.stores[0].get::<Entity>(index) };
                                entities
                                    .get_datum_mut_unchecked(moved)
                                    .update(index as u32, segment.index as u32);
                            }
                        }
                    }
                }
                Defer::All(_) => {
                    for segment in query.segments() {
                        let segment = &mut world.segments[segment];
                        if segment.count > 0 {
                            entities
                                .release(unsafe { segment.stores[0].get_all(0, segment.count) });
                            segment.clear();
                        }
                    }
                }
            }
        }
    }
}

impl<'a, I: Item + 'static, F: Filter> Get<'a> for State<I, F> {
    type Item = Destroy<'a, I, F>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy {
            defer: &mut self.defer,
            query: self.query.get(world),
        }
    }
}

unsafe impl<I: Item, F: Filter> Depend for State<I, F> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for segment in self.query.inner.as_ref().segments() {
            dependencies.push(Dependency::Defer(segment, TypeId::of::<Entity>()));
        }
        dependencies
    }
}
