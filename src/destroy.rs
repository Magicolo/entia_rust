use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entity::Entity,
    filter::Filter,
    inject::{Context, Get, Inject},
    item::Item,
    query::{self, Query},
    world::World,
};
use std::{any::TypeId, marker::PhantomData};

pub struct Destroy<'a, I: Item + 'static = (), F: Filter = ()>(Defer<'a, Destruction<I, F>>);
pub struct State<I: Item + 'static, F: Filter>(defer::State<Destruction<I, F>>);

enum Destruction<I: Item, F: Filter> {
    One(Entity),
    All(PhantomData<(I, F)>),
}

impl<I: Item, F: Filter> Destroy<'_, I, F> {
    #[inline]
    pub fn one(&mut self, entity: Entity) {
        self.0.defer(Destruction::One(entity));
    }

    #[inline]
    pub fn filter_one(&mut self, entity: Entity, filter: impl FnMut(I) -> bool) {
        todo!()
    }

    #[inline]
    pub fn many(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.one(entity);
        }
    }

    #[inline]
    pub fn filter_many(
        &mut self,
        entities: impl IntoIterator<Item = Entity>,
        filter: impl FnMut(I) -> bool,
    ) {
        todo!()
    }

    #[inline]
    pub fn all(&mut self) {
        self.0.defer(Destruction::All(PhantomData));
    }

    #[inline]
    pub fn filter_all(&mut self, filter: impl FnMut(I) -> bool) {
        todo!()
    }
}

impl<I: Item, F: Filter> Inject for Destroy<'_, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let input = <Query<Entity, F> as Inject>::initialize((), context, world)?;
        let defer = <Defer<Destruction<I, F>> as Inject>::initialize(input, context, world)?;
        Some(State(defer))
    }

    fn update(State(state): &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::update(state.as_mut(), world);
        <Defer<Destruction<I, F>> as Inject>::update(state, world);
    }

    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::resolve(state.as_mut(), world);
        <Defer<Destruction<I, F>> as Inject>::resolve(state, world);
    }
}

impl<I: Item, F: Filter> Clone for State<I, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, I: Item + 'static, F: Filter> Get<'a> for State<I, F> {
    type Item = Destroy<'a, I, F>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy(self.0.get(world))
    }
}

unsafe impl<I: Item, F: Filter> Depend for State<I, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let query = self.0.as_ref().inner.as_ref();
        for segment in &query.segments {
            dependencies.push(Dependency::Defer(segment, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<I: Item + 'static, F: Filter> Resolve for Destruction<I, F> {
    type State = query::State<Entity, F>;

    fn resolve(items: impl Iterator<Item = Self>, state: &mut Self::State, world: &mut World) {
        let entities = state.entities.as_mut();
        let query = state.inner.as_mut();
        for item in items {
            match item {
                Destruction::One(entity) => {
                    if let Some(datum) = entities.get_datum_mut(entity) {
                        let index = datum.index() as usize;
                        let segment = datum.segment() as usize;
                        if query.segments.has(segment) {
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
                Destruction::All(_) => {
                    for (item, segment) in query.states.iter_mut() {
                        let segment = &mut world.segments[*segment];
                        if segment.count > 0 {
                            entities.release(unsafe { item.0.get_all(0, segment.count) });
                            segment.clear();
                        }
                    }
                }
            }
        }
    }
}
