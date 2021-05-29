use entia_core::{bits::Bits, utility::get_mut2, Change};

use crate::{
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    modify::Modify,
    resource::Resource,
    segment::Move,
    system::Dependency,
    world::{Store, World},
    write::{self, Write},
};
use std::{any::TypeId, marker::PhantomData, sync::Arc};

pub struct Remove<'a, M: Modify, F: Filter = ()>(&'a mut Inner<M, F>);

pub struct State<M: Modify, F: Filter> {
    inner: write::State<Inner<M, F>>,
    entities: entities::State,
}

enum Target {
    Invalid,
    Pending(Bits),
    Valid(Arc<Store<Entity>>, Move),
}

struct Inner<M: Modify, F: Filter> {
    all: bool,
    defer: Vec<Entity>,
    targets: Vec<Target>,
    _marker: PhantomData<(M, F)>,
}

impl<M: Modify, F: Filter> Resource for Inner<M, F> {}

impl<M: Modify, F: Filter> Default for Inner<M, F> {
    fn default() -> Self {
        Self {
            all: false,
            defer: Vec::new(),
            targets: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<M: Modify, F: Filter> Remove<'_, M, F> {
    #[inline]
    pub fn remove(&mut self, entity: Entity) {
        self.0.defer.push(entity);
    }

    #[inline]
    pub fn remove_all(&mut self) {
        self.0.all = true;
    }
}

impl<M: Modify, F: Filter> Inject for Remove<'_, M, F> {
    type Input = ();
    type State = State<M, F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        let inner = <Write<Inner<M, F>> as Inject>::initialize(None, world)?;
        let entities = <Entities as Inject>::initialize((), world)?;
        Some(State { inner, entities })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        let inner = state.inner.as_mut();
        while let Some(source) = world.segments.get(inner.targets.len()) {
            let validate = || {
                source.static_store::<Entity>()?;
                M::initialize(source, world).filter(|_| F::filter(source, world))?;
                Some((source.index, source.types.clone()))
            };

            let target = validate()
                .map(|(source, mut types)| {
                    for meta in M::static_metas(world) {
                        types.set(meta.index, false);
                    }

                    let mut complete = || {
                        let target = world.get_segment_by_types(&types)?;
                        let store = target.static_store()?;
                        let target = target.index;
                        let (source, target) = get_mut2(&mut world.segments, (source, target))?;
                        Some(Target::Valid(store, source.prepare_move(target)))
                    };
                    complete().unwrap_or(Target::Pending(types))
                })
                .unwrap_or(Target::Invalid);
            inner.targets.push(target);
        }
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        Self::update(state, world);

        fn complete(source: usize, types: Bits, world: &mut World) -> Option<Target> {
            let target = world.get_or_add_segment_by_types(&types, None);
            let store = target.static_store()?;
            let target = target.index;
            let (source, target) = get_mut2(&mut world.segments, (source, target))?;
            Some(Target::Valid(store, source.prepare_move(target)))
        }

        fn get<'a>(source: usize, targets: &'a mut Vec<Target>, world: &mut World) -> &'a Target {
            let target = &mut targets[source];
            match target {
                Target::Pending(types) => {
                    *target = complete(source, types.clone(), world).unwrap_or(Target::Invalid);
                }
                _ => {}
            };
            target
        }

        let entities = &mut state.entities;
        let state = state.inner.as_mut();
        if state.all.change(false) {
            state.defer.clear();

            for i in 0..state.targets.len() {
                match get(i, &mut state.targets, world) {
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
        } else {
            for entity in state.defer.drain(..) {
                if let Some(datum) = entities.get_datum_mut(entity) {
                    let index = datum.index as usize;
                    let source = datum.segment as usize;
                    match get(source, &mut state.targets, world) {
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
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = <Write<Inner<M, F>> as Inject>::depend(&state.inner, world);
        for target in state.inner.as_ref().targets.iter() {
            match target {
                // No need to check 'source != target' since 'get_mut2' already does this check.
                Target::Valid(_, target) => {
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
        Remove(self.inner.get(world))
    }
}
