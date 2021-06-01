use std::any::TypeId;

use entia_core::One;

use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::{Homogeneous, Modify},
    segment::Store,
    world::World,
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
    One(Entity, M),
    Many(Box<[Entity]>, Box<[M]>),
    Clone(Box<[Entity]>, M, fn(&M) -> M),
}

impl<M: Modify> Create<'_, M> {
    pub fn create(&mut self, modify: M) -> Entity {
        let mut entities = [Entity::ZERO];
        self.entities.reserve(&mut entities);
        self.defer.defer(Creation::One(entities[0], modify));
        entities[0]
    }

    pub fn create_many(&mut self, modifies: Box<[M]>) -> Box<[Entity]>
    where
        M: Homogeneous,
    {
        if modifies.len() == 0 {
            [].into()
        } else if modifies.len() == 1 {
            [self.create(modifies.one().unwrap())].into()
        } else {
            let entities = self.reserve(modifies.len());
            let defer = Creation::Many(entities.clone().into(), modifies);
            self.defer.defer(defer);
            entities
        }
    }

    pub fn create_clone<const N: usize>(&mut self, modify: M, count: usize) -> Box<[Entity]>
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

    pub fn create_default<const N: usize>(&mut self, count: usize) -> Box<[Entity]>
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

impl<M: Modify> Depend for State<M> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.defer.depend(world);
        for &(_, _, target) in self.defer.as_ref().1.iter() {
            // No need to consider 'M::depend' since the entity's components can not be seen from other threads until 'resolve' is called.
            // Only the less constraining 'Add' dependency is required to ensure consistency.
            dependencies.push(Dependency::Defer(target, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<M: Modify> Resolve for Creation<M> {
    type State = (entities::State, Vec<(M::State, Store, usize)>);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let entities = <Entities as Inject>::initialize((), world)?;
        Some((entities, Vec::new()))
    }

    fn resolve(self, (entities, targets): &mut Self::State, world: &mut World) {
        match self {
            Creation::One(entity, modify) => {
                let target = targets
                    .iter()
                    .position(|pair| modify.validate(&pair.0))
                    .or_else(|| {
                        let meta = world.get_or_add_meta::<Entity>();
                        let mut metas = vec![meta.clone()];
                        metas.append(&mut modify.dynamic_metas(world));
                        let target = world.get_or_add_segment_by_metas(metas, None).index;
                        let target = &world.segments[target];
                        let entities = unsafe { target.store(&meta)?.clone() };
                        let state = M::initialize(target, world)?;
                        let index = targets.len();
                        targets.push((state, entities, target.index));
                        return Some(index);
                    })
                    .and_then(|index| targets.get_mut(index));

                if let Some(datum) = entities.get_datum_mut(entity) {
                    if let Some((state, entities, target)) = target {
                        let target = &mut world.segments[*target];
                        let index = target.reserve(1);
                        unsafe { entities.set(index, entity) };
                        modify.modify(state, index);
                        datum.initialize(index as u32, target.index as u32);
                    }
                }
            }
            Creation::Many(entities, modifies) => {}
            Creation::Clone(entities, modify, clone) => {}
        }
    }
}
