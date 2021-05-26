use entia_core::{bits::Bits, utility::get_mut2, Change};

use crate::{
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Get, Inject},
    modify::Modify,
    segment::Move,
    system::Dependency,
    world::{Store, World},
};
use std::{any::TypeId, marker::PhantomData, sync::Arc};

pub struct Remove<'a, M: Modify, F: Filter = ()> {
    all: &'a mut bool,
    defer: &'a mut Vec<Entity>,
    _marker: PhantomData<(M, F)>,
}

pub struct State<M: Modify, F: Filter> {
    all: bool,
    defer: Vec<Entity>,
    targets: Vec<Target>,
    entities: entities::State,
    _marker: PhantomData<(M, F)>,
}

enum Target {
    Invalid,
    Pending(Bits),
    Valid(Arc<Store<Entity>>, Move),
}

impl<M: Modify, F: Filter> Remove<'_, M, F> {
    pub fn remove(&mut self, entity: Entity) {
        // TODO: Try to optimisticaly resolve here.
        self.defer.push(entity);
    }

    pub fn remove_all(&mut self) {
        *self.all = true;
    }
}

impl<M: Modify + 'static, F: Filter + 'static> Inject for Remove<'_, M, F> {
    type Input = ();
    type State = State<M, F>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            all: false,
            defer: Vec::new(),
            targets: Vec::new(),
            entities: state,
            _marker: PhantomData,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(source) = world.segments.get(state.targets.len()) {
            let validate = || {
                let meta = world.get_meta::<Entity>()?;
                M::initialize(source, world)
                    .filter(|_| F::filter(source, world) && source.types.has(meta.index))?;
                Some((source.index, source.types.clone()))
            };

            let target = validate()
                .map(|(source, mut types)| {
                    for meta in M::static_metas(world) {
                        types.remove(meta.index);
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
            state.targets.push(target);
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

        if state.all.change(false) {
            state.defer.clear();

            for i in 0..state.targets.len() {
                match get(i, &mut state.targets, world) {
                    Target::Valid(store, target) => {
                        let count = world.segments[i].count;
                        if let Some(index) = target.apply(0, count, world) {
                            for i in index..index + count {
                                let entity = *unsafe { store.at(i) };
                                let datum = unsafe {
                                    state.entities.get_datum_at_mut(entity.index as usize)
                                };
                                datum.index = i as u32;
                                datum.segment = target.target() as u32;
                                datum.store = store.clone();
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            for entity in state.defer.drain(..) {
                if let Some(datum) = state.entities.get_datum_mut(entity) {
                    let index = datum.index as usize;
                    let source = datum.segment as usize;
                    match get(source, &mut state.targets, world) {
                        Target::Valid(store, target) => {
                            if let Some(index) = target.apply(index, 1, world) {
                                datum.index = index as u32;
                                datum.segment = target.target() as u32;
                                datum.store = store.clone();
                            }
                        }
                        _ => {}
                    };
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for target in state.targets.iter() {
            match target {
                // No need to check 'source != target' since 'get_mut2' already does this check.
                Target::Valid(_, target) => {
                    dependencies.push(Dependency::Defer(target.target(), TypeId::of::<Entity>()))
                }
                _ => {}
            };
        }
        dependencies
    }
}

impl<'a, M: Modify + 'static, F: Filter + 'static> Get<'a> for State<M, F> {
    type Item = Remove<'a, M, F>;

    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Remove {
            all: &mut self.all,
            defer: &mut self.defer,
            _marker: PhantomData,
        }
    }
}
