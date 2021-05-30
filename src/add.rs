use entia_core::utility::get_mut2;

use crate::{
    defer::{self, Defer, Resolve},
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::Modify,
    segment::Move,
    system::Dependency,
    world::World,
};
use std::{any::TypeId, collections::HashMap};

pub struct Add<'a, M: Modify>(Defer<'a, Addition<M>>);

pub struct State<M: Modify> {
    index: usize,
    defer: defer::State<Addition<M>>,
}

struct Addition<M: Modify>(Entity, M);

impl<M: Modify> Add<'_, M> {
    #[inline]
    pub fn add(&mut self, entity: Entity, modify: M) {
        self.0.defer(Addition(entity, modify));
    }
}

impl<M: Modify> Inject for Add<'_, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let defer = <Defer<Addition<M>> as Inject>::initialize((), world)?;
        Some(State { index: 0, defer })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Defer<Addition<M>> as Inject>::update(&mut state.defer, world);
        // TODO: Segments must be collected in advance to ensure the order of 'Add' when no resolution have occured.
        /*
        system1: [Add<Position>]
        system2: [Add<Position, Option<Velocity>>]
        - both systems add to the same entity
        - both systems initially have no dependencies
        - which position does the entity have?
        - since 'resolve' is called sequentially in order of system declaration, system2 will win, preserving coherence!
        - this means that 'Add' only has 'Add' dependencies on target segments as long as its resolution is deferred

        - internal dependencies must be checked though

        Add<C>, Query<Not<C>>:
        - If 'Add' depends only on target segments, it will not conflict with the 'Query' when it should.
        - If concurrent, should an empty entity be seen by the 'Query'?
        - To fix this, 'Add' would need an 'Add(Entity)' dependency on every entity segment.

        */
        while let Some(segment) = world.segments.get(state.index) {
            state.index += 1;

            // - check for entity store
            if let Some(modify) = M::initialize(segment, world) {
                // state.targets
            }
        }
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        <Defer<Addition<M>> as Inject>::resolve(&mut state.defer, world);
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = <Defer<Addition<M>> as Inject>::depend(&state.defer, world);
        let (_, targets) = state.defer.as_ref();
        for pair in targets.values().flat_map(|targets| targets) {
            dependencies.push(Dependency::Defer(pair.1.target(), TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, M: Modify> Get<'a> for State<M> {
    type Item = Add<'a, M>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Add(self.defer.get(world))
    }
}

impl<M: Modify> Resolve for Addition<M> {
    type State = (entities::State, HashMap<usize, Vec<(M::State, Move)>>);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let entities = <Entities as Inject>::initialize((), world)?;
        Some((entities, HashMap::new()))
    }

    fn resolve(self, (entities, targets): &mut Self::State, world: &mut World) {
        let Self(entity, modify) = self;
        if let Some(datum) = entities.get_datum_mut(entity) {
            let index = datum.index as usize;
            let source = datum.segment as usize;
            let targets = targets.entry(source).or_insert_with(|| {
                let mut targets = Vec::new();
                let source = &world.segments[source];
                if let Some(state) = M::initialize(source, world) {
                    targets.push((state, source.prepare_move(source)));
                }
                targets
            });
            let target = targets
                .iter()
                .position(|pair| modify.validate(&pair.0))
                .or_else(|| {
                    let mut types = world.segments[source].types.clone();
                    for meta in modify.dynamic_metas(world) {
                        types.set(meta.index, true);
                    }

                    let target = world.get_or_add_segment_by_types(&types, None).index;
                    let state = M::initialize(&world.segments[target], world)?;
                    let indices = (source, target);
                    let (source, target) = get_mut2(&mut world.segments, indices)?;
                    let index = targets.len();
                    targets.push((state, source.prepare_move(target)));
                    return Some(index);
                })
                .and_then(|index| targets.get_mut(index));

            if let Some(target) = target {
                if let Some(index) = target.1.apply(index, 1, world) {
                    modify.modify(&target.0, index);
                    datum.index = index as u32;
                    datum.segment = target.1.target() as u32;
                }
            }
        }
    }
}
