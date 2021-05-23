use crate::{
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    modify::Modify,
    system::Dependency,
    world::World,
};
use std::{any::TypeId, collections::HashMap, marker::PhantomData};

pub struct Destroy<'a, M: Modify = ()> {
    defer: &'a mut Vec<Entity>,
    _marker: PhantomData<M>,
}

pub struct State<M: Modify> {
    defer: Vec<Entity>,
    targets: HashMap<usize, bool>,
    entities: entities::State,
    _marker: PhantomData<M>,
}

impl<M: Modify> Destroy<'_, M> {
    pub fn destroy(&mut self, entity: Entity) {
        // TODO: Try to optimisticaly resolve here.
        self.defer.push(entity);
    }
}

impl<M: Modify + 'static> Inject for Destroy<'_, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            defer: Vec::new(),
            targets: HashMap::new(),
            entities: state,
            _marker: PhantomData,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let mut entities = state.entities.entities();
        for entity in state.defer.drain(..) {
            if let Some(datum) = entities.get_datum_mut(entity) {
                let index = datum.index as usize;
                let source = datum.segment as usize;
                let targets = &mut state.targets;
                let target = targets
                    .entry(source)
                    .or_insert_with(|| M::initialize(&world.segments[source], world).is_some());

                if *target {
                    world.segments[source].clear_at(index);
                    entities.release(&[entity]);
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (&target, _) in state.targets.iter().filter(|(_, &value)| value) {
            dependencies.push(Dependency::Write(target, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a, M: Modify + 'static> Get<'a> for State<M> {
    type Item = Destroy<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Destroy {
            defer: &mut self.defer,
            _marker: PhantomData,
        }
    }
}
