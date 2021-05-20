use entia_core::utility::get_mut2;

use crate::component::*;
use crate::entities;
use crate::entities::*;
use crate::entity::*;
use crate::inject::*;
use crate::segment::*;
use crate::system::*;
use crate::world::*;
use std::collections::HashMap;
use std::{any::TypeId, sync::Arc};

/*
Add<(Position, Velocity)>({ source: [target] })
- Dependencies:
    - for each source that has 1+ targets: Write(source, Entity)
    - for each target: [Add(target, Entity), Write(target, Position), Write(target, Velocity)]
- May move an entity from a source segment (segments that do not have both [Position, Velocity]) to a target segment (segments that do have
both [Position, Velocity] and that has a link with a source segment)
- Note that segment [Position, Velocity, Status] is only a target segment if there is a source segment [Status], [Position, Status],
[Velocity, Status], otherwise, it is not a valid target since the addition of the specified components cannot lead to it.
- Note that only segment with an entity store can be depended on.
- When calling 'Add::add(self, entity, initialize)':
    let datum = get_datum(entity);
    let source = datum.segment;
    if let Some(targets) = self.source_to_targets.get(source) {
        let target = initialize.select_candidate(targets);
        if source == target {
            // write components to current segment
            initialize.initialize(source, datum.index, 1);
        } else {
            // move entity from 'source' to 'target'
            self.defer(initialize, source, target, datum.index, 1);
        }
    }
*/

pub struct Add<'a, M: Modify> {
    defer: &'a mut Vec<(Entity, M)>,
}

pub struct State<M: Modify> {
    targets: HashMap<usize, Vec<(M::State, Move)>>,
    defer: Vec<(Entity, M)>,
    entities: entities::State,
}

pub trait Modify<M = ()> {
    type State;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn metas(&self, world: &mut World) -> Vec<Meta>;
    fn validate(&self, state: &Self::State) -> bool;
    fn modify(self, state: &Self::State, index: usize);
    fn depend(state: &Self::State) -> Vec<Dependency>;
}

impl<C: Component> Modify<[(); 0]> for C {
    type State = (Arc<Store<C>>, usize);

    fn initialize(segment: &Segment, _: &World) -> Option<Self::State> {
        Some((segment.static_store()?, segment.index))
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        vec![world.get_or_add_meta::<C>()]
    }

    #[inline]
    fn validate(&self, _: &Self::State) -> bool {
        true
    }

    #[inline]
    fn modify(self, (store, _): &Self::State, index: usize) {
        *unsafe { store.at(index) } = self;
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<C>())]
    }
}

impl<C: Component, F: FnOnce() -> C> Modify<[C; 1]> for F {
    type State = (Arc<Store<C>>, usize);

    fn initialize(segment: &Segment, _: &World) -> Option<Self::State> {
        Some((segment.static_store()?, segment.index))
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        vec![world.get_or_add_meta::<C>()]
    }

    #[inline]
    fn validate(&self, _: &Self::State) -> bool {
        true
    }

    #[inline]
    fn modify(self, (store, _): &Self::State, index: usize) {
        *unsafe { store.at(index) } = self();
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(state.1, TypeId::of::<C>())]
    }
}

impl<M: Modify> Modify for Option<M> {
    type State = M::State;

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        M::initialize(segment, world)
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        match self {
            Some(modify) => modify.metas(world),
            None => Vec::new(),
        }
    }

    #[inline]
    fn validate(&self, state: &Self::State) -> bool {
        match self {
            Some(modify) => modify.validate(state),
            _ => false,
        }
    }

    #[inline]
    fn modify(self, state: &Self::State, index: usize) {
        match self {
            Some(modify) => modify.modify(state, index),
            _ => {}
        }
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        M::depend(state)
    }
}

impl<M1: Modify, M2: Modify> Modify for (M1, M2) {
    type State = (M1::State, M2::State);

    fn initialize(segment: &Segment, world: &World) -> Option<Self::State> {
        Some((
            M1::initialize(segment, world)?,
            M2::initialize(segment, world)?,
        ))
    }

    fn metas(&self, world: &mut World) -> Vec<Meta> {
        let mut metas = Vec::new();
        metas.append(&mut self.0.metas(world));
        metas.append(&mut self.1.metas(world));
        metas
    }

    #[inline]
    fn validate(&self, state: &Self::State) -> bool {
        self.0.validate(&state.0) && self.1.validate(&state.1)
    }

    #[inline]
    fn modify(self, state: &Self::State, index: usize) {
        self.0.modify(&state.0, index);
        self.1.modify(&state.1, index);
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        dependencies.append(&mut M1::depend(&state.0));
        dependencies.append(&mut M2::depend(&state.1));
        dependencies
    }
}

impl<M: Modify> Add<'_, M> {
    pub fn add(&mut self, entity: Entity, modify: M) {
        self.defer.push((entity, modify));
    }

    //     fn try_resolve(&self, index: usize, source: usize, modify: &M) -> Resolution {
    //         if let Some(targets) = self.targets.get(&source) {
    //             for i in 0..targets.len() {
    //                 let pair = &targets[i];
    //                 if modify.validate(&pair.0) {
    //                     if pair.1.source() == pair.1.target() {
    //                         return Resolution::Resolve(&pair.0);
    //                     } else {
    //                         return Resolution::Defer(i);
    //                     }
    //                 }
    //             }
    //         }
    //         None
    //     }

    //     fn select<'a>(
    //         targets: &'a mut HashMap<usize, Vec<(M::State, Move)>>,
    //         modify: &M,
    //         source: usize,
    //         world: &mut World,
    //     ) -> Option<&'a (M::State, Move)> {
    //         let targets = targets.entry(source).or_default();
    //         let mut index = None;
    //         for i in 0..targets.len() {
    //             let pair = &targets[i];
    //             if modify.validate(&pair.0) {
    //                 index = Some(i);
    //             }
    //         }

    //         match index {
    //             Some(index) => Some(&targets[index]),
    //             None => {
    //                 let mut types = world.segments[source].types.clone();
    //                 for meta in modify.metas(world) {
    //                     types.add(meta.index);
    //                 }

    //                 let target = world.get_or_add_segment_by_types(&types, None).index;
    //                 if let Some(state) = M::initialize(&world.segments[target], world) {
    //                     let indices = (source, target);
    //                     if let Some((source, target)) = get_mut2(&mut world.segments, indices) {
    //                         targets.push((state, source.prepare_move(target)));
    //                         return targets.last();
    //                     }
    //                 }
    //                 None
    //             }
    //         }
    //     }
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
                let pair = &targets[i];
                if modify.validate(&pair.0) {
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

impl<'a, T: Modify + 'static> Get<'a> for State<T> {
    type Item = Add<'a, T>;

    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Add {
            defer: &mut self.defer,
        }
    }
}
