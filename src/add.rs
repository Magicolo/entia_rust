use crate::core::utility::*;
use crate::entities;
use crate::entities::*;
use crate::entity::*;
use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::collections::HashMap;

/*
Add<(Position, Velocity)>({ source: [target] })
- Dependencies:
    - for each source that has 1+ targets: Write(source, Entity)
    - for each target: [Add(target, Entity), Write(target, Position), Write(target, Velocity)]
- May move an entity from a source segment (segments that do not have both [Position, Velocity]) to a target segment (segments that do have
both [Position, Velocity] and that has a link with a source segment)
- Note that segment [Position, Velocity, Status] is only a target segment if there is a source segment [Status], [Position, Status],
[Velocity, Status], otherwise, it is not a valid target since the addition of the specified components cannot lead to it.
- Note that only segment with an entity store can be depended on.
- When calling 'Add::add(self, entity, initialize)':
    let datum = get_datum(entity);
    let source = datum.segment;
    if let Some(targets) = self.source_to_targets.get(source) {
        let target = initialize.select_candidate(targets);
        if source == target {
            // write components to current segment
            initialize.initialize(source, datum.index, 1);
        } else {
            // move entity from 'source' to 'target'
            self.defer(initialize, source, target, datum.index, 1);
        }
    }
*/

pub trait Boba {
    fn is_target_candidate(segment: &Segment) -> bool;
    fn can_move_to(source: &Segment, target: &Segment) -> bool;
    fn is_target(&self, segment: &Segment) -> bool;
    fn add_target<'a>(&self, world: &'a mut World) -> &'a mut Segment;
    fn initialize(self, index: usize, segment: &Segment);
    fn depend(segment: &Segment) -> Vec<Dependency>;
}

pub struct Add<'a, T: Boba>(
    &'a mut Vec<(Entity, T, usize, Option<usize>, usize)>,
    &'a HashMap<usize, Vec<usize>>,
    Entities<'a>,
    &'a World,
);

pub struct State<T>(
    usize,
    Vec<(Entity, T, usize, Option<usize>, usize)>,
    HashMap<usize, Vec<usize>>,
    entities::State,
);

impl<T: Boba> Add<'_, T> {
    pub fn add(&mut self, entity: Entity, initialize: T) -> bool {
        fn select<'a, T: Boba>(
            initialize: &T,
            candidates: &Vec<usize>,
            world: &'a World,
        ) -> Option<&'a Segment> {
            for &candidate in candidates {
                let segment = &world.segments[candidate];
                if initialize.is_target(segment) {
                    return Some(segment);
                }
            }
            None
        }

        if let Some(datum) = self.2.get_datum(entity) {
            let index = datum.index as usize;
            let source = datum.segment as usize;
            let targets = &self.1[&source]; // All entity segments must be in the hash map.

            if let Some(target) = select(&initialize, targets, self.3) {
                if source == target.index {
                    // Entity is already in the target segment, so simply write the data.
                    initialize.initialize(index, target);
                } else {
                    // Entity will need to be moved to a new segment, so defer.
                    self.0
                        .push((entity, initialize, source, Some(target.index), index));
                }
            } else {
                self.0.push((entity, initialize, source, None, index));
            }

            true
        } else {
            false
        }
    }
}

impl<T: Boba + 'static> Inject for Add<'_, T> {
    type Input = ();
    type State = State<T>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        Entities::initialize((), world).map(|state| State(0, Vec::new(), HashMap::new(), state))
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.0) {
            state.0 += 1;

            let mut targets = Vec::new();
            if T::is_target_candidate(segment) {
                // TODO: Map source segments to target segments.
                for (&source, targets) in state.2.iter_mut() {
                    if T::can_move_to(&world.segments[source], segment) {
                        targets.push(segment.index);
                    }
                }

                for source in world.segments.iter() {
                    if T::can_move_to(source, segment) {
                        targets.push(source.index);
                    }
                }
            }

            state.2.insert(segment.index, targets);
        }
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        // TODO: Cache mapping of source segment stores to target segment stores?
        for (entity, initialize, source, target, index) in state.1.drain(..) {
            // If target segment does not exist yet, so create it.
            let target = match target {
                Some(target) => target,
                None => initialize.add_target(world).index,
            };

            let mut entities = state.3.entities();
            if let Some((source, target)) = get_mut2(&mut world.segments, (source, target)) {
                if let Some(datum) = entities.get_datum_mut(entity) {
                    if let Some(index) = source.move_to(index, target) {
                        datum.index = index as u32;
                        datum.segment = target.index as u32;
                        initialize.initialize(index, &target);
                    }
                }
            }
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (&source, targets) in state.2.iter() {
            if targets.len() > 0 {
                dependencies.push(Dependency::Write(source, TypeId::of::<Entity>()));
                for &target in targets {
                    dependencies.push(Dependency::Add(target, TypeId::of::<Entity>()));
                    dependencies.append(&mut T::depend(&world.segments[target]));
                }
            }
        }
        dependencies
    }
}

impl<'a, T: Boba + 'static> Get<'a> for State<T> {
    type Item = Add<'a, T>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Add(&mut self.1, &self.2, self.3.get(world), world)
    }
}
