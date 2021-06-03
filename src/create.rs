use std::{any::TypeId, intrinsics::transmute, mem::ManuallyDrop, ops::Deref, sync::Arc};

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

    pub fn create_many(&mut self, modifies: Box<[M]>) -> &[Entity]
    where
        M: Homogeneous,
    {
        if modifies.len() == 0 {
            return &[];
        }

        let entities = self.reserve(modifies.len());
        let defer = Creation::Many(entities, modifies);
        if let Some(Creation::Many(entities, ..)) = self.defer.defer(defer) {
            &entities
        } else {
            unreachable!()
        }
    }

    pub fn create_clone(&mut self, modify: M, count: usize) -> &[Entity]
    where
        M: Clone,
    {
        if count == 0 {
            return &[];
        }

        let entities = self.reserve(count);
        let defer = Creation::Clone(entities.clone().into(), modify, M::clone);
        if let Some(Creation::Clone(entities, ..)) = self.defer.defer(defer) {
            &entities
        } else {
            unreachable!()
        }
    }

    pub fn create_default(&mut self, count: usize) -> &[Entity]
    where
        M: Default,
    {
        if count == 0 {
            return &[];
        }

        let modify = Default::default();
        let entities = self.reserve(count);
        let defer = Creation::Clone(entities.clone().into(), modify, |_| Default::default());
        if let Some(Creation::Clone(entities, ..)) = self.defer.defer(defer) {
            &entities
        } else {
            unreachable!()
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
    type State = (entities::State, Vec<(M::State, Arc<Store>, usize)>);

    fn initialize(world: &mut World) -> Option<Self::State> {
        let entities = <Entities as Inject>::initialize((), world)?;
        Some((entities, Vec::new()))
    }

    fn resolve(self, (entities, targets): &mut Self::State, world: &mut World) {
        fn find<'a, M: Modify>(
            modify: &M,
            targets: &'a mut Vec<(M::State, Arc<Store>, usize)>,
            world: &mut World,
        ) -> Option<&'a mut (M::State, Arc<Store>, usize)> {
            let index = targets
                .iter()
                .position(|pair| modify.validate(&pair.0))
                .or_else(|| {
                    let meta = world.get_or_add_meta::<Entity>();
                    let mut metas = vec![meta.clone()];
                    metas.append(&mut modify.dynamic_metas(world));
                    let target = world.get_or_add_segment_by_metas(metas, None).index;
                    let target = &world.segments[target];
                    let entities = target.store(&meta)?;
                    let state = M::initialize(target, world)?;
                    let index = targets.len();
                    targets.push((state, entities, target.index));
                    return Some(index);
                })?;
            targets.get_mut(index)
        }

        // The entities can be assumed have not been destroyed since this operation has been enqueued before any other
        // operation that could concern them.
        match self {
            Creation::One(entity, modify) => {
                if let Some((state, store, target)) = find(&modify, targets, world) {
                    let target = &mut world.segments[*target];
                    let index = target.reserve(1);
                    unsafe { store.set(index, &[entity]) };
                    modify.modify(state, index);
                    let datum = entities.get_datum_at_mut(entity.index as usize);
                    datum.initialize(index as u32, target.index as u32);
                }
            }
            Creation::Many(many, modifies) => {
                let mut modifies: Box<[ManuallyDrop<M>]> = unsafe { transmute(modifies) };
                let modify = modifies[0].deref();
                if let Some((state, store, target)) = find(modify, targets, world) {
                    let target = &mut world.segments[*target];
                    let index = target.reserve(many.len());
                    unsafe { store.set(index, &many) };

                    for i in 0..many.len() {
                        let entity = many[i];
                        let modify = unsafe { ManuallyDrop::take(&mut modifies[i]) };
                        modify.modify(state, index);
                        let datum = entities.get_datum_at_mut(entity.index as usize);
                        datum.initialize(index as u32, target.index as u32);
                    }
                }
            }
            Creation::Clone(many, modify, clone) => {
                if let Some((state, store, target)) = find(&modify, targets, world) {
                    let target = &mut world.segments[*target];
                    let index = target.reserve(many.len());
                    unsafe { store.set(index, &many) };

                    for i in 0..many.len() {
                        let entity = many[i];
                        let modify = clone(&modify);
                        modify.modify(state, index);
                        let datum = entities.get_datum_at_mut(entity.index as usize);
                        datum.initialize(index as u32, target.index as u32);
                    }
                }
            }
        }
    }
}
