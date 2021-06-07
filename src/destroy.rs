use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entity::Entity,
    filter::Filter,
    inject::{Context, Get, Inject},
    query::{self, Query},
    world::World,
};
use std::{any::TypeId, marker::PhantomData};

pub struct Destroy<'a, F: Filter = ()>(Defer<'a, Destruction<F>>);
pub struct State<F: Filter>(defer::State<Destruction<F>>);

enum Destruction<F: Filter> {
    One(Entity),
    All(PhantomData<F>),
}

impl<F: Filter> Destroy<'_, F> {
    #[inline]
    pub fn destroy(&mut self, entity: Entity) {
        self.0.defer(Destruction::One(entity));
    }

    #[inline]
    pub fn destroy_many(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.destroy(entity);
        }
    }

    #[inline]
    pub fn destroy_all(&mut self) {
        self.0.defer(Destruction::All(PhantomData));
    }
}

impl<F: Filter> Inject for Destroy<'_, F> {
    type Input = ();
    type State = State<F>;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let input = <Query<Entity, F> as Inject>::initialize((), context, world)?;
        let defer = <Defer<Destruction<F>> as Inject>::initialize(input, context, world)?;
        Some(State(defer))
    }

    fn update(State(state): &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::update(state.as_mut(), world);
        <Defer<Destruction<F>> as Inject>::update(state, world);
    }

    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::resolve(state.as_mut(), world);
        <Defer<Destruction<F>> as Inject>::resolve(state, world);
    }
}

impl<F: Filter> Clone for State<F> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, F: Filter> Get<'a> for State<F> {
    type Item = Destroy<'a, F>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy(self.0.get(world))
    }
}

unsafe impl<F: Filter> Depend for State<F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let query = self.0.as_ref().inner.as_ref();
        for segment in &query.segments {
            dependencies.push(Dependency::Defer(segment, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<F: Filter> Resolve for Destruction<F> {
    type State = query::State<Entity, F>;

    fn resolve(items: impl Iterator<Item = Self>, state: &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::update(state, world);

        let entities = &mut state.entities;
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
                                unsafe { entities.get_datum_mut_unchecked(moved) }
                                    .update(index as u32, segment.index as u32);
                            }
                        }
                    }
                }
                Destruction::All(_) => {
                    for (item, segment, _) in query.states.iter_mut() {
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
