use crate::component::Metadata;
use crate::inject::*;
use crate::*;
use crossbeam_queue::SegQueue;

// HOW ARE SYSTEMS RUN?
// By default, all systems are run every time the world runs.
// To run conditionnaly, a system may wrap another and add a condition such as:
// - run only when a message is received (initialization and finalization should be handled this way)
// - run only when a given resource exists
// - run only when at least 1 entity corresponds to a query
// - run when a given amount of time has elapsed
// More generally, systems will only run when their dependencies are satisfied.

// WILL SYSTEMS RUN IN PARRALEL?
// By default, all systems are run in parallel unless their dependencies collide.
// The world will offer a bunch of unsafe operations and will have to be used with care.
// Systems will use the unsafe operations of the world in a safe way by managing dependencies between systems.

// WILL EACH SYSTEMS RUN IN PARRALEL?
// By default, all 'each' systems will also run in parallel if the dependencies allows it.
// For example, a systems that creates/destroys entities will not allow parallel execution (and will also prevent
// other systems to run parallel to it), but if that system defers those operations instead, then it
// will be allowed.

// HOW DOES THE DEFER MODULE WORK?
// The 'Defer' module will have to be thread safe and will also create a synchronization point when it resolves.
// A user should be allowed to determine when the defer module resolves.
// This may be accomplished by taking a '&mut' reference of the 'Defer' module which would give access to the
// 'resolve' method and create a synchronization point.
// A convenient 'Resolve' system should be provided such that it can be easily inserted in an execution schedule.
// By default, at the end of 1 execution frame, the world will resolve all deferred operations (note that a user
// should be allowed to opt-out of this behavior).

// HOW DO I KNOW WHY A SYSTEM HAS OR HASN'T RUN?
// At some point, there should be a mechanism to 'explain' the execution or non-execution of a system.
// This would, of course, be a debug-only feature but should allow to solve bugs or simply understand
// how entia works.

// WHERE ARE RESOURCES STORED?
// Ideally, resources should simply be stored on a special resource entity. This way, they will get
// treated as any other component. This also means that internally, there should be not difference
// between resources and components and that these constraints should only come at the API level.
// The special entity would hold one of each existing resources.

// WHERE ARE EMITTERS/RECEIVERS STORED?
// Similarly to resources, emitters/receivers should be stored on special entities.
// Ideally, all state is stored on some entity somewhere.
// Alternatively, there could be no emitters at all since all they do it iterate on all entities with
// the component 'Receiver<T>' and enqueue some data on their thread safe queue. Therefore, they can
// simply be implemented as a 'each' system wrapped in an 'Emitter<T>' module for convenience.

pub trait System<'a, P = ()> {
    type State: 'a;

    fn state(world: &mut World) -> Option<Self::State>;
    fn run(&self, state: &'a mut Self::State, world: &'a World);
}

struct State<'a, P, S: System<'a, P>>(S, S::State);

trait Runner<'a> {
    fn run(&'a mut self, world: &'a World);
}

impl<'a, P, S: System<'a, P>> Runner<'a> for State<'a, P, S> {
    fn run(&'a mut self, world: &'a world::World) {
        self.0.run(&mut self.1, world);
    }
}

impl World {
    fn add_system<'a, P, S: System<'a, P>>(&'a mut self, system: S) -> bool {
        let mut runners: Vec<Box<dyn Runner<'a>>> = Vec::new();
        if let Some(state) = S::state(self) {
            runners.push(Box::new(State(system, state)));
            true
        } else {
            false
        }
    }
}

pub struct Position(f32, f32);
pub struct Velocity(f32, f32);
pub struct Receiver<T>(SegQueue<T>);
pub enum Status {
    None,
    Frozen,
    Poisoned,
    Invincible,
}
impl Component for Position {
    fn metadata() -> &'static Metadata {
        todo!()
    }
}
impl Component for Velocity {
    fn metadata() -> &'static component::Metadata {
        todo!()
    }
}
impl<T> Component for Receiver<T> {
    fn metadata() -> &'static component::Metadata {
        todo!()
    }
}
impl Component for Status {
    fn metadata() -> &'static component::Metadata {
        todo!()
    }
}

pub struct Time;
pub struct Physics;
impl Resource for Time {}
impl Resource for Physics {}
pub struct OnKill(Entity);

pub fn test_main(world: &mut World) {
    loop {
        world.add_system(|(_, _): (Entity, &Position)| {});
        world.add_system(|_: &Time, (_, _): (Entity, &Position)| {});
        world.add_system(|_: &mut Time, _: &Position| {});
        world.add_system(
            |(_, _): (&Time, &mut Physics), (position, velocity): (&mut Position, &Velocity)| {
                position.0 += velocity.0;
                position.1 += velocity.1;
            },
        );
        // world.add_system(|group: &Group<(&mut Position, &Velocity)>| {
        //     // for (position, velocity) in group.iter() {
        //     //     position.0 += velocity.0;
        //     //     position.1 += velocity.1;
        //     // }
        // });
        world.add_system(
            |(_, _): (&Time, &Physics), (_, _, _): (Entity, &Position, Option<&Status>)| {},
        );
        world.add_system(|_: &inject::Entities| {});
        world.add_system(|_: &mut Defer| {});

        // Emit system
        world.add_system(|receiver: &Receiver<OnKill>| {
            receiver.0.push(OnKill(Entity::ZERO));
        });
    }
}

impl<'a, I: Inject<'a>, F: Fn(I)> System<'a, [I; 0]> for F {
    type State = I::State;

    fn state(world: &mut World) -> Option<Self::State> {
        I::state(world)
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        self(I::inject(state, world));
    }
}

impl<'a, Q: Query<'a> + 'a, F: Fn(Q)> System<'a, [Q; 1]> for F {
    type State = <&'a Group<'a, Q> as Inject<'a>>::State;

    fn state(world: &mut World) -> Option<Self::State> {
        <&Group<Q> as Inject>::state(world)
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        let group = <&Group<Q> as Inject>::inject(state, world);
        for (index, state) in &group.segments {
            let segment = &world.segments[*index];
            for i in 0..segment.entities.len() {
                self(Q::query(i, state, segment));
            }
        }
    }
}

impl<'a, I: Inject<'a>, Q: Query<'a> + 'a, F: Fn(I, Q)> System<'a, [(I, Q); 2]> for F {
    type State = (I::State, <&'a Group<'a, Q> as Inject<'a>>::State);

    fn state(world: &mut World) -> Option<Self::State> {
        match (I::state(world), <&Group<Q> as Inject>::state(world)) {
            (Some(inject), Some(group)) => Some((inject, group)),
            _ => None,
        }
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        let inject = I::inject(&mut state.0, world);
        let group = <&Group<Q> as Inject>::inject(&mut state.1, world);
        for (index, query) in &group.segments {
            let segment = &world.segments[*index];
            for i in 0..segment.entities.len() {
                self(inject.copy(), Q::query(i, query, segment));
            }
        }
    }
}
