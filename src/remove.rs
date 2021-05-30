use entia_core::{bits::Bits, utility::get_mut2};

use crate::{
    defer::{self, Defer, Resolve},
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    modify::Modify,
    segment::{Move, Segment},
    system::Dependency,
    world::{Store, World},
};
use std::{any::TypeId, marker::PhantomData, sync::Arc};

pub struct Remove<'a, M: Modify, F: Filter = ()>(Defer<'a, Removal<M, F>>);
pub struct State<M: Modify, F: Filter>(defer::State<Removal<M, F>>);

enum Target {
    Invalid,
    Pending(Bits),
    Valid(Arc<Store<Entity>>, Move),
}

enum Removal<M: Modify, F: Filter> {
    One(Entity),
    All(PhantomData<(M, F)>),
}

impl<M: Modify, F: Filter> Remove<'_, M, F> {
    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        self.0.defer(Removal::One(entity));
    }

    #[inline]
    pub fn remove_all(&mut self) {
        self.0.defer(Removal::All(PhantomData));
    }
}

impl<M: Modify, F: Filter> Inject for Remove<'_, M, F> {
    type Input = ();
    type State = State<M, F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let defer = <Defer<Removal<M, F>> as Inject>::initialize((), world)?;
        Some(State(defer))
    }

    fn update(State(state): &mut Self::State, world: &mut World) {
        <Defer<Removal<M, F>> as Inject>::update(state, world);
    }

    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Defer<Removal<M, F>> as Inject>::resolve(state, world);
    }

    fn depend(State(state): &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = <Defer<Removal<M, F>> as Inject>::depend(state, world);
        for target in state.as_ref().1.iter() {
            match target {
                Target::Valid(_, target) if target.source() != target.target() => {
                    // A 'Read' from 'source' after a 'Remove' must not see removed entities.
                    dependencies.push(Dependency::Defer(target.source(), TypeId::of::<Entity>()));
                    // A 'Read' from 'target' after a 'Remove' must see removed entities.
                    dependencies.push(Dependency::Defer(target.target(), TypeId::of::<Entity>()));
                }
                _ => {}
            };
        }
        dependencies
    }
}

impl<'a, M: Modify, F: Filter> Get<'a> for State<M, F> {
    type Item = Remove<'a, M, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Remove(self.0.get(world))
    }
}

impl<M: Modify, F: Filter> Resolve for Removal<M, F> {
    type State = (entities::State, Vec<Target>);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let entities = <Entities as Inject>::initialize((), world)?;
        Some((entities, Vec::new()))
    }

    fn resolve(self, (entities, targets): &mut Self::State, world: &mut World) {
        fn validate<M: Modify, F: Filter>(
            source: &Segment,
            world: &World,
        ) -> Option<(usize, Bits)> {
            source.static_store::<Entity>()?;
            M::initialize(source, world).filter(|_| F::filter(source, world))?;
            Some((source.index, source.types.clone()))
        }

        fn complete(source: usize, types: &Bits, add: bool, world: &mut World) -> Option<Target> {
            let target = if add {
                world.get_or_add_segment_by_types(types, None)
            } else {
                world.get_segment_by_types(types)?
            };
            let store = target.static_store()?;
            let target = target.index;
            let (source, target) = get_mut2(&mut world.segments, (source, target))?;
            Some(Target::Valid(store, source.prepare_move(target)))
        }

        fn get<'a>(
            source: usize,
            targets: &'a mut Vec<Target>,
            world: &mut World,
        ) -> &'a mut Target {
            let target = &mut targets[source];
            match target {
                Target::Pending(types) => {
                    *target = complete(source, types, true, world).unwrap_or(Target::Invalid);
                }
                _ => {}
            };
            target
        }

        while let Some(source) = world.segments.get(targets.len()) {
            let target = validate::<M, F>(source, world)
                .map(|(source, mut types)| {
                    for meta in M::static_metas(world) {
                        types.set(meta.index, false);
                    }
                    complete(source, &types, false, world).unwrap_or(Target::Pending(types))
                })
                .unwrap_or(Target::Invalid);
            targets.push(target);
        }

        match self {
            Removal::One(entity) => {
                if let Some(datum) = entities.get_datum_mut(entity) {
                    let index = datum.index as usize;
                    let source = datum.segment as usize;
                    match get(source, targets, world) {
                        Target::Valid(_, target) => {
                            if let Some(index) = target.apply(index, 1, world) {
                                datum.index = index as u32;
                                datum.segment = target.target() as u32;
                            }
                        }
                        _ => {}
                    };
                }
            }
            Removal::All(_) => {
                for i in 0..targets.len() {
                    match get(i, targets, world) {
                        Target::Valid(store, target) => {
                            let count = world.segments[i].count;
                            if let Some(index) = target.apply(0, count, world) {
                                for i in index..index + count {
                                    let entity = *unsafe { store.at(i) };
                                    if let Some(datum) =
                                        entities.get_datum_at_mut(entity.index as usize)
                                    {
                                        datum.index = i as u32;
                                        datum.segment = target.target() as u32;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
