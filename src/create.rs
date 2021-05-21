use std::{any::TypeId, sync::Arc};

use crate::{
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::Modify,
    system::Dependency,
    world::{Store, World},
};

pub struct Create<'a, M: Modify> {
    defer: &'a mut Vec<(Entity, M)>,
    entities: Entities<'a>,
}

pub struct State<M: Modify> {
    defer: Vec<(Entity, M)>,
    targets: Vec<(M::State, Arc<Store<Entity>>, usize)>,
    entities: entities::State,
}

impl<M: Modify> Create<'_, M> {
    pub fn create(&mut self, modify: M) -> Entity {
        let entities: [Entity; 1] = self.entities.create();
        let entity = entities[0];
        self.defer.push((entity, modify));
        entity
    }
}

impl<M: Modify + 'static> Inject for Create<'_, M> {
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
        fn select<'a, M: Modify>(
            targets: &'a mut Vec<(M::State, Arc<Store<Entity>>, usize)>,
            modify: &M,
            world: &mut World,
        ) -> Option<&'a (M::State, Arc<Store<Entity>>, usize)> {
            let mut index = None;
            for i in 0..targets.len() {
                let (state, _, _) = &targets[i];
                if modify.validate(state) {
                    index = Some(i);
                }
            }

            match index {
                Some(index) => Some(&targets[index]),
                None => {
                    let mut metas = vec![world.get_or_add_meta::<Entity>()];
                    metas.append(&mut modify.metas(world));
                    let target = world.get_or_add_segment_by_metas(&metas, None).index;
                    if let Some(entities) = world.segments[target].static_store() {
                        if let Some(state) = M::initialize(&world.segments[target], world) {
                            targets.push((state, entities, target));
                            return targets.last();
                        }
                    }
                    None
                }
            }
        }

        for (entity, modify) in state.defer.drain(..) {
            if let Some((state, entities, target)) = select(&mut state.targets, &modify, world) {
                let target = &mut world.segments[*target];
                let index = target.reserve(1);
                *unsafe { entities.at(index) } = entity;
                modify.modify(state, index);
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for &(_, _, target) in state.targets.iter() {
            // No need to consider 'M::depend' since the entity's components can not be seen from other threads until 'resolve' is called.
            // Only the lightweight 'Add' dependency is required to ensure consistency.
            dependencies.push(Dependency::Add(target, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, M: Modify + 'static> Get<'a> for State<M> {
    type Item = Create<'a, M>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            defer: &mut self.defer,
            entities: self.entities.get(world),
        }
    }
}
