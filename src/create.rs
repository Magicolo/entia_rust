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

// TODO: add create_batch
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
        /*
        CREATE:
        static: (Position, Option<Velocity>)
        dynamic: (Position, None)
        targets: []
        metas: [Position]
        -> add_segment, move

        static: (Position, Option<Velocity>)
        dynamic: (Position, Some(Velocity))
        targets: [[Position]]
        metas: [Position, Velocity]
        -> add_segment, move

        // Validate should ensure that 'target.types == Bits(metas)'

        ADD:
        // Validate should ensure that 'target.types.has_all(Bits(metas))'
        */
        for (entity, modify) in state.defer.drain(..) {
            let targets = &mut state.targets;
            let target = targets
                .iter()
                .position(|pair| modify.validate(&pair.0))
                .map_or_else(
                    || {
                        let mut metas = vec![world.get_or_add_meta::<Entity>()];
                        metas.append(&mut modify.metas(world));
                        let target = world.get_or_add_segment_by_metas(&metas, None).index;
                        let entities = world.segments[target].static_store()?;
                        let state = M::initialize(&world.segments[target], world)?;
                        targets.push((state, entities, target));
                        return targets.last();
                    },
                    |index| Some(&state.targets[index]),
                );

            if let Some((state, entities, target)) = target {
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
