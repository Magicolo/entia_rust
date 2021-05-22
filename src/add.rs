use entia_core::utility::get_mut2;

use crate::{
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::Modify,
    segment::Move,
    system::Dependency,
    world::World,
};
use std::{any::TypeId, collections::HashMap};

pub struct Add<'a, M: Modify> {
    defer: &'a mut Vec<(Entity, M)>,
}

pub struct State<M: Modify> {
    targets: HashMap<usize, Vec<(M::State, Move)>>,
    defer: Vec<(Entity, M)>,
    entities: entities::State,
}

impl<M: Modify> Add<'_, M> {
    pub fn add(&mut self, entity: Entity, modify: M) {
        // TODO: Try to optimisticaly resolve the 'add' here.
        self.defer.push((entity, modify));
    }
}

impl<M: Modify + 'static> Inject for Add<'_, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            targets: HashMap::new(),
            defer: Vec::new(),
            entities: state,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let mut entities = state.entities.entities();
        for (entity, modify) in state.defer.drain(..) {
            if let Some(datum) = entities.get_datum_mut(entity) {
                let index = datum.index as usize;
                let source = datum.segment as usize;
                let targets = state.targets.entry(source).or_insert_with(|| {
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
                            types.add(meta.index);
                        }

                        let target = world.get_or_add_segment_by_types(&types, None).index;
                        let state = M::initialize(&world.segments[target], world)?;
                        let indices = (source, target);
                        let (source, target) = get_mut2(&mut world.segments, indices)?;
                        let index = targets.len();
                        targets.push((state, source.prepare_move(target)));
                        return Some(index);
                    })
                    .and_then(|index| targets.get(index));

                if let Some(target) = target {
                    if let Some(index) = target.1.apply(index, world) {
                        modify.modify(&target.0, index);
                        datum.index = index as u32;
                        datum.segment = target.1.target() as u32;
                    }
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for pair in state.targets.values().flat_map(|targets| targets) {
            if pair.1.source() != pair.1.target() {
                dependencies.push(Dependency::Write(pair.1.source(), TypeId::of::<Entity>()));
                dependencies.push(Dependency::Add(pair.1.target(), TypeId::of::<Entity>()));
            }
            dependencies.append(&mut M::depend(&pair.0));
        }

        dependencies
    }
}

impl<'a, M: Modify + 'static> Get<'a> for State<M> {
    type Item = Add<'a, M>;

    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Add {
            defer: &mut self.defer,
        }
    }
}
