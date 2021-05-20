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

pub struct Add<'a, M: Modify> {
    defer: &'a mut Vec<(Entity, M)>,
}

pub struct State<M: Modify> {
    targets: HashMap<usize, Vec<(M::State, Move)>>,
    defer: Vec<(Entity, M)>,
    entities: entities::State,
}

pub trait Modify {
    type State;
    fn initialize(segment: &Segment, world: &World) -> Option<Self::State>;
    fn metas(&self, world: &mut World) -> Vec<Meta>;
    fn validate(&self, state: &Self::State) -> bool;
    fn modify(self, state: &Self::State, index: usize);
    fn depend(state: &Self::State) -> Vec<Dependency>;
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

impl<C: Component> Modify for C {
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

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Modify,)*> Modify for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_segment: &Segment, _world: &World) -> Option<Self::State> {
                Some(($($t::initialize(_segment, _world)?,)*))
            }

            fn metas(&self, _world: &mut World) -> Vec<Meta> {
                let ($($p,)*) = self;
                let mut _metas = Vec::new();
                $(_metas.append(&mut $p.metas(_world));)*
                _metas
            }

            #[inline]
            fn validate(&self, ($($p,)*): &Self::State) -> bool {
                let ($($t,)*) = self;
                $($t.validate($p) && )* true
            }

            #[inline]
            fn modify(self, ($($p,)*): &Self::State, _index: usize) {
                let ($($t,)*) = self;
                $($t.modify($p, _index);)*
            }

            fn depend(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::depend($p));)*
                _dependencies
            }
        }
    };
}

entia_macro::recurse_32!(modify);
