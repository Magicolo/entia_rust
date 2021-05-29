use std::{any::TypeId, sync::Arc};

use entia_core::One;

use crate::{
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::{Homogeneous, Modify},
    system::Dependency,
    world::{Store, World},
};

pub struct Create<'a, M: Modify> {
    defer: &'a mut Vec<Defer<M>>,
    entities: Entities<'a>,
}

pub struct State<M: Modify> {
    defer: Vec<Defer<M>>,
    targets: Vec<(M::State, Arc<Store<Entity>>, usize)>,
    entities: entities::State,
}

enum Defer<M> {
    Single(Entity, M),
    Batch(Box<[Entity]>, Box<[M]>),
    Clone(Box<[Entity]>, M, fn(&M) -> M),
}

impl<M: Modify> Create<'_, M> {
    pub fn create(&mut self, modify: M) -> Entity {
        let mut entities = [Entity::ZERO];
        self.entities.reserve(&mut entities);
        self.defer.push(Defer::Single(entities[0], modify));
        entities[0]
    }

    pub fn create_all(&mut self, modifies: Box<[M]>) -> Box<[Entity]>
    where
        M: Homogeneous,
    {
        if modifies.len() == 0 {
            [].into()
        } else if modifies.len() == 1 {
            [self.create(modifies.one().unwrap())].into()
        } else {
            let entities = self.reserve(modifies.len());
            let defer = Defer::Batch(entities.clone().into(), modifies);
            self.defer.push(defer);
            entities
        }
    }

    pub fn create_all_clone<const N: usize>(&mut self, modify: M, count: usize) -> Box<[Entity]>
    where
        M: Clone + Homogeneous,
    {
        if count == 0 {
            [].into()
        } else if count == 1 {
            [self.create(modify)].into()
        } else {
            let entities = self.reserve(count);
            let defer = Defer::Clone(entities.clone().into(), modify, Clone::clone);
            self.defer.push(defer);
            entities
        }
    }

    pub fn create_all_default<const N: usize>(&mut self, count: usize) -> Box<[Entity]>
    where
        M: Default + Homogeneous,
    {
        if count == 0 {
            [].into()
        } else if count == 1 {
            [self.create(M::default())].into()
        } else {
            let entities = self.reserve(count);
            let defer = Defer::Clone(entities.clone().into(), M::default(), |_| M::default());
            self.defer.push(defer);
            entities
        }
    }

    fn reserve(&mut self, count: usize) -> Box<[Entity]> {
        let mut entities = Vec::with_capacity(count);
        unsafe { entities.set_len(count) };
        self.entities.reserve(&mut entities);
        entities.into_boxed_slice()
    }
}

impl<M: Modify> Inject for Create<'_, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            defer: Vec::new(),
            targets: Vec::new(),
            entities: state,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        for defer in state.defer.drain(..) {
            match defer {
                Defer::Single(entity, modify) => {
                    let targets = &mut state.targets;
                    let target = targets
                        .iter()
                        .position(|pair| modify.validate(&pair.0))
                        .or_else(|| {
                            let mut metas = vec![world.get_or_add_meta::<Entity>()];
                            metas.append(&mut modify.dynamic_metas(world));
                            let target = world.get_or_add_segment_by_metas(&metas, None).index;
                            let target = &world.segments[target];
                            let entities = target.static_store()?;
                            let state = M::initialize(target, world)?;
                            let index = targets.len();
                            targets.push((state, entities, target.index));
                            return Some(index);
                        })
                        .and_then(|index| targets.get(index));

                    if let Some((state, entities, target)) = target {
                        let target = &mut world.segments[*target];
                        let index = target.reserve(1);
                        *unsafe { entities.at(index) } = entity;
                        modify.modify(state, index);
                    }
                }
                Defer::Batch(entities, modifies) => {}
                Defer::Clone(entities, modify, clone) => {}
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for &(_, _, target) in state.targets.iter() {
            // No need to consider 'M::depend' since the entity's components can not be seen from other threads until 'resolve' is called.
            // Only the less constraining 'Add' dependency is required to ensure consistency.
            dependencies.push(Dependency::Defer(target, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, M: Modify> Get<'a> for State<M> {
    type Item = Create<'a, M>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            defer: &mut self.defer,
            entities: self.entities.get(world),
        }
    }
}
