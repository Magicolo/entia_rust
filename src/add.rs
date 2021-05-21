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
        fn select<'a, M: Modify>(
            targets: &'a mut HashMap<usize, Vec<(M::State, Move)>>,
            modify: &M,
            source: usize,
            world: &mut World,
        ) -> Option<&'a (M::State, Move)> {
            let targets = targets.entry(source).or_default();
            let mut index = None;
            for i in 0..targets.len() {
                let (state, _) = &targets[i];
                if modify.validate(state) {
                    index = Some(i);
                }
            }

            match index {
                Some(index) => Some(&targets[index]),
                None => {
                    let mut types = world.segments[source].types.clone();
                    for meta in modify.metas(world) {
                        types.add(meta.index);
                    }

                    let target = world.get_or_add_segment_by_types(&types, None).index;
                    if let Some(state) = M::initialize(&world.segments[target], world) {
                        let indices = (source, target);
                        if let Some((source, target)) = get_mut2(&mut world.segments, indices) {
                            targets.push((state, source.prepare_move(target)));
                            return targets.last();
                        }
                    }
                    None
                }
            }
        }

        for (entity, modify) in state.defer.drain(..) {
            let mut entities = state.entities.entities();
            if let Some(datum) = entities.get_datum_mut(entity) {
                let index = datum.index as usize;
                let source = datum.segment as usize;
                if let Some(target) = select(&mut state.targets, &modify, source, world) {
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

        for (_, targets) in state.targets.iter() {
            for (state, motion) in targets {
                if motion.source() != motion.target() {
                    dependencies.push(Dependency::Write(motion.source(), TypeId::of::<Entity>()));
                    dependencies.push(Dependency::Add(motion.target(), TypeId::of::<Entity>()));
                }
                dependencies.append(&mut M::depend(state));
            }
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
