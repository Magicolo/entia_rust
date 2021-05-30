use std::{any::TypeId, sync::Arc};

use entia_core::One;

use crate::{
    defer::{self, Defer, Resolve},
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::{Homogeneous, Modify},
    system::Dependency,
    world::{Store, World},
};

pub struct Create<'a, M: Modify> {
    defer: Defer<'a, Creation<M>>,
    entities: Entities<'a>,
}

pub struct State<M: Modify> {
    defer: defer::State<Creation<M>>,
    entities: entities::State,
}

enum Creation<M> {
    Single(Entity, M),
    Batch(Box<[Entity]>, Box<[M]>),
    Clone(Box<[Entity]>, M, fn(&M) -> M),
}

impl<M: Modify> Create<'_, M> {
    pub fn create(&mut self, modify: M) -> Entity {
        let mut entities = [Entity::ZERO];
        self.entities.reserve(&mut entities);
        self.defer.defer(Creation::Single(entities[0], modify));
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
            let defer = Creation::Batch(entities.clone().into(), modifies);
            self.defer.defer(defer);
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
            let defer = Creation::Clone(entities.clone().into(), modify, Clone::clone);
            self.defer.defer(defer);
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
            let defer = Creation::Clone(entities.clone().into(), M::default(), |_| M::default());
            self.defer.defer(defer);
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
        let defer = <Defer<Creation<M>> as Inject>::initialize((), world)?;
        let entities = <Entities as Inject>::initialize((), world)?;
        Some(State { defer, entities })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Defer<Creation<M>> as Inject>::update(&mut state.defer, world);
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        <Defer<Creation<M>> as Inject>::resolve(&mut state.defer, world);
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = <Defer<Creation<M>> as Inject>::depend(&state.defer, world);
        for &(_, _, target) in state.defer.as_ref().iter() {
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
            defer: self.defer.get(world),
            entities: self.entities.get(world),
        }
    }
}

impl<M: Modify> Resolve for Creation<M> {
    type State = Vec<(M::State, Arc<Store<Entity>>, usize)>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(Vec::new())
    }

    fn resolve(self, targets: &mut Self::State, world: &mut World) {
        match self {
            Creation::Single(entity, modify) => {
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
            Creation::Batch(entities, modifies) => {}
            Creation::Clone(entities, modify, clone) => {}
        }
    }
}
