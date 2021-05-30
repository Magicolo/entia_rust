use crate::{
    defer::{self, Defer, Resolve},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    query::{self, Query},
    system::Dependency,
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
    pub fn destroy_all(&mut self) {
        self.0.defer(Destruction::All(PhantomData));
    }
}

impl<F: Filter> Inject for Destroy<'_, F> {
    type Input = ();
    type State = State<F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let defer = <Defer<Destruction<F>> as Inject>::initialize((), world)?;
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

    fn depend(State(state): &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = <Defer<Destruction<F>> as Inject>::depend(state, world);
        let query = state.as_ref().inner.as_ref();
        for segment in &query.segments {
            dependencies.push(Dependency::Defer(segment, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, F: Filter> Get<'a> for State<F> {
    type Item = Destroy<'a, F>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy(self.0.get(world))
    }
}

impl<F: Filter> Resolve for Destruction<F> {
    type State = query::State<Entity, F>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        <Query<Entity, F> as Inject>::initialize((), world)
    }

    fn resolve(self, state: &mut Self::State, world: &mut World) {
        let entities = &mut state.entities;
        let query = state.inner.as_mut();
        match self {
            Destruction::One(entity) => {
                if let Some(datum) = entities.get_datum_mut(entity) {
                    let index = datum.index as usize;
                    let segment = datum.segment as usize;
                    if query.segments.has(segment) {
                        entities.release(&[entity]);
                        world.segments[segment].clear_at(index);
                    }
                }
            }
            Destruction::All(_) => {
                for (item, segment, count) in query.states.iter() {
                    let count = *count;
                    if count > 0 {
                        entities.release(&unsafe { item.0.get() }[0..count]);
                        world.segments[*segment].clear();
                    }
                }
            }
        }
    }
}
