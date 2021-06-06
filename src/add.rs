use entia_core::utility::get_mut2;

use crate::{
    defer::{self, Defer, Resolve},
    depend::{Depend, Dependency},
    entities::{self, Entities},
    entity::Entity,
    filter::Filter,
    inject::{Context, Get, Inject},
    modify::Modify,
    segment::Move,
    world::World,
};
use std::{any::TypeId, collections::HashMap, marker::PhantomData};

pub struct Add<'a, M: Modify, F: Filter = ()>(Defer<'a, Addition<M, F>>);

pub struct State<M: Modify, F: Filter> {
    index: usize,
    defer: defer::State<Addition<M, F>>,
}

enum Addition<M: Modify, F: Filter> {
    /// Adds the components described by 'M' to the given entity.
    One(Entity, M, PhantomData<F>),
    /// Adds the components described by 'M' to all entities that correspond to the filter 'F'.
    All(M, fn(&M) -> M),
}

impl<M: Modify, F: Filter> Add<'_, M, F> {
    #[inline]
    pub fn add(&mut self, entity: Entity, modify: M) {
        self.0.defer(Addition::One(entity, modify, PhantomData));
    }

    #[inline]
    pub fn add_all_clone(&mut self, modify: M)
    where
        M: Clone,
    {
        self.0.defer(Addition::All(modify, M::clone));
    }

    pub fn add_all_default(&mut self)
    where
        M: Default,
    {
        self.0.defer(Addition::All(M::default(), |_| M::default()));
    }
}

impl<M: Modify, F: Filter> Inject for Add<'_, M, F> {
    type Input = ();
    type State = State<M, F>;

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let entities = <Entities as Inject>::initialize((), context, world)?;
        let input = (entities, HashMap::new());
        let defer = <Defer<Addition<M, F>> as Inject>::initialize(input, context, world)?;
        Some(State { index: 0, defer })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        <Defer<Addition<M, F>> as Inject>::update(&mut state.defer, world);
        // TODO: Segments must be collected in advance to ensure the order of 'Add' when no resolution have occured.
        /*
        system1: [Add<Position>]
        system2: [Add<Position, Option<Velocity>>]
        - both systems add to the same entity
        - both systems initially have no dependencies
        - which position does the entity have?
        - since 'resolve' is called sequentially in order of system declaration, system2 will win, preserving coherence!
        - this means that 'Add' only has 'Add' dependencies on target segments as long as its resolution is deferred

        - internal dependencies must be checked though

        Add<C>, Query<Not<C>>:
        - If 'Add' depends only on target segments, it will not conflict with the 'Query' when it should.
        - If concurrent, should an empty entity be seen by the 'Query'?
        - To fix this, 'Add' would need an 'Add(Entity)' dependency on every entity segment.

        */
        while let Some(segment) = world.segments.get(state.index) {
            state.index += 1;

            // - check for entity store
            if let Some(_) = M::initialize(segment, world) {
                todo!()
                // state.targets
            }
        }
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        <Defer<Addition<M, F>> as Inject>::resolve(&mut state.defer, world);
    }
}

impl<M: Modify, F: Filter> Clone for State<M, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            defer: self.defer.clone(),
            index: self.index,
        }
    }
}

impl<'a, M: Modify, F: Filter> Get<'a> for State<M, F> {
    type Item = Add<'a, M, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Add(self.defer.get(world))
    }
}

unsafe impl<M: Modify, F: Filter> Depend for State<M, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.defer.depend(world);
        let (_, targets) = self.defer.as_ref();
        for pair in targets.values().flat_map(|targets| targets) {
            dependencies.push(Dependency::Defer(pair.1.target(), TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<M: Modify, F: Filter> Resolve for Addition<M, F> {
    type State = (entities::State, HashMap<usize, Vec<(M::State, Move)>>);

    fn resolve(
        items: impl Iterator<Item = Self>,
        (entities, targets): &mut Self::State,
        world: &mut World,
    ) {
        for item in items {
            match item {
                Addition::One(entity, modify, _) => {
                    if let Some(datum) = entities.get_datum_mut(entity) {
                        let index = datum.index() as usize;
                        let source = datum.segment() as usize;
                        let targets = targets.entry(source).or_insert_with(|| {
                            let mut targets = Vec::new();
                            let source = &world.segments[source];
                            if let Some(state) = Some(source)
                                .filter(|segment| F::filter(segment, world))
                                .and_then(|segment| M::initialize(segment, world))
                            {
                                targets.push((state, Move::new(source, source)));
                            }
                            targets
                        });

                        let target = targets
                            .iter()
                            .position(|pair| modify.validate(&pair.0))
                            .or_else(|| {
                                let mut types = world.segments[source].types.clone();
                                for meta in modify.dynamic_metas(world) {
                                    types.set(meta.index, true);
                                }

                                let target = world.get_or_add_segment_by_types(&types, None).index;
                                let state = Some(&world.segments[target])
                                    .filter(|segment| F::filter(segment, world))
                                    .and_then(|segment| M::initialize(segment, world))?;
                                let indices = (source, target);
                                let (source, target) = get_mut2(&mut world.segments, indices)?;
                                let index = targets.len();
                                targets.push((state, Move::new(source, target)));
                                return Some(index);
                            })
                            .and_then(|index| targets.get_mut(index));

                        if let Some(target) = target {
                            if let Some(index) = target.1.apply(index, 1, world) {
                                modify.modify(&mut target.0, index);
                                datum.update(index as u32, target.1.target() as u32);
                            }
                        }
                    }
                }
                Addition::All(..) => {
                    todo!()
                }
            }
        }
    }
}
