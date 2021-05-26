use entia_core::Change;

use crate::{
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    query::{self, Query},
    system::Dependency,
    world::World,
};
use std::{any::TypeId, marker::PhantomData};

pub struct Destroy<'a, F: Filter = ()> {
    all: &'a mut bool,
    defer: &'a mut Vec<Entity>,
    _marker: PhantomData<F>,
}

pub struct State<F: Filter> {
    all: bool,
    defer: Vec<Entity>,
    query: query::State<Entity, F>,
}

impl<F: Filter> Destroy<'_, F> {
    pub fn destroy(&mut self, entity: Entity) {
        // TODO: Try to optimisticaly resolve here.
        self.defer.push(entity);
    }

    pub fn destroy_all(&mut self) {
        *self.all = true;
    }
}

impl<F: Filter + 'static> Inject for Destroy<'_, F> {
    type Input = ();
    type State = State<F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Query<Entity, F> as Inject>::initialize((), world).map(|state| State {
            all: false,
            defer: Vec::new(),
            query: state,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Query<Entity, F> as Inject>::update(&mut state.query, world)
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        if state.all.change(false) {
            state.defer.clear();
            for (item, segment, count) in state.query.states.iter() {
                let count = *count;
                if count > 0 {
                    state
                        .query
                        .entities
                        .release(&unsafe { item.0.get() }[0..count]);
                    world.segments[*segment].clear();
                }
            }
        } else {
            for entity in state.defer.drain(..) {
                if let Some(datum) = state.query.entities.get_datum_mut(entity) {
                    let index = datum.index as usize;
                    let segment = datum.segment as usize;
                    if state.query.segments.has(segment) {
                        world.segments[segment].clear_at(index);
                        state.query.entities.release(&[entity]);
                    }
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for segment in &state.query.segments {
            dependencies.push(Dependency::Defer(segment, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, F: Filter + 'static> Get<'a> for State<F> {
    type Item = Destroy<'a, F>;

    #[inline]
    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Destroy {
            all: &mut self.all,
            defer: &mut self.defer,
            _marker: PhantomData,
        }
    }
}
