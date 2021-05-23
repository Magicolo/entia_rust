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
use std::{any::TypeId, collections::HashMap, marker::PhantomData};

pub struct Remove<'a, M: Modify> {
    defer: &'a mut Vec<Entity>,
    _marker: PhantomData<M>,
}

pub struct State<M: Modify> {
    targets: HashMap<usize, Option<Move>>,
    defer: Vec<Entity>,
    entities: entities::State,
    _marker: PhantomData<M>,
}

impl<M: Modify> Remove<'_, M> {
    // TODO: add 'remove_batch'
    pub fn remove(&mut self, entity: Entity) {
        // TODO: Try to optimisticaly resolve here.
        self.defer.push(entity);
    }
}

impl<M: Modify + 'static> Inject for Remove<'_, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            targets: HashMap::new(),
            defer: Vec::new(),
            entities: state,
            _marker: PhantomData,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let mut entities = state.entities.entities();
        for entity in state.defer.drain(..) {
            if let Some(datum) = entities.get_datum_mut(entity) {
                let index = datum.index as usize;
                let source = datum.segment as usize;
                let target = state
                    .targets
                    .entry(source)
                    .or_insert_with(|| {
                        M::initialize(&world.segments[source], world)?;
                        let mut types = world.segments[source].types.clone();
                        for meta in M::static_metas(world) {
                            types.remove(meta.index);
                        }

                        let target = world.get_or_add_segment_by_types(&types, None).index;
                        let indices = (source, target);
                        let (source, target) = get_mut2(&mut world.segments, indices)?;
                        Some(source.prepare_move(target))
                    })
                    .as_ref();

                if let Some(target) = target {
                    if let Some(index) = target.apply(index, world) {
                        datum.index = index as u32;
                        datum.segment = target.target() as u32;
                    }
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for target in state.targets.values().filter_map(Option::as_ref) {
            if target.source() != target.target() {
                dependencies.push(Dependency::Write(target.source(), TypeId::of::<Entity>()));
                dependencies.push(Dependency::Add(target.target(), TypeId::of::<Entity>()));
            }
        }
        dependencies
    }
}

impl<'a, M: Modify + 'static> Get<'a> for State<M> {
    type Item = Remove<'a, M>;

    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Remove {
            defer: &mut self.defer,
            _marker: PhantomData,
        }
    }
}
