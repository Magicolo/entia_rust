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

// CAN A RESOURCE BE MUTATED IN A QUERY SYSTEM?
// Currently, this is disallowed since a query system that also has injectables requires the 'Copy' trait
// on the injectable.
// For this restriction to be lifted, the query system would have to be guaranteed to run sequentially when
// a write access to an injectable is detected.
// Ideally this would be manageable by the user and enforced by the type system.

pub trait System<'a, P = ()> {
    type State: 'a;

    fn state(world: &'a mut World) -> Option<Self::State>;
    fn run(&self, state: &'a mut Self::State, world: &'a World);
}

struct Systems<T> {
    systems: T,
}

struct Runner<'a, P, S: System<'a, P>> {
    state: S::State,
    system: S,
}

impl Systems<()> {
    fn new() -> Self {
        Self { systems: () }
    }
}

impl<T> Systems<T> {
    fn add<'a, P, S: System<'a, P>>(self, system: S) -> Systems<(T, S)> {
        todo!()
    }

    // fn schedule(self, _: &mut World) -> Runner<'a, P, S> {
    //     todo!()
    // }
}

impl<'a, P, S: System<'a, P>> Runner<'a, P, S> {
    // Take a 'mut' reference of the world such that this runner is the only thing operating on the world since
    // there is unsafe code in the implementations of systems that assume that dependencies have been checked.
    fn run(&'a mut self, world: &'a mut World) {
        self.system.run(&mut self.state, world);
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
impl Component for Position {}
impl Component for Velocity {}
impl<T> Component for Receiver<T> {}
impl Component for Status {}

pub struct Time;
pub struct Physics;
impl Resource for Time {}
impl Resource for Physics {}
pub struct OnKill(Entity);

pub fn test_main(world: &mut World) {
    Systems::new()
        .add(|(_, _): (Entity, &Position)| {})
        .add(|_: &Time, (_, _): (Entity, &Position)| {})
        .add(|_: &Time, _: &Position| {})
        .add(
            |(_, _): (&Time, &Physics), (position, velocity): (&mut Position, &Velocity)| {
                position.0 += velocity.0;
                position.1 += velocity.1;
            },
        )
        // .add(|group: &Group<(&mut Position, &Velocity)>| {
        //     // for (position, velocity) in group.iter() {
        //     //     position.0 += velocity.0;
        //     //     position.1 += velocity.1;
        //     // }
        // })
        .add(|(_, _): (&Time, &Physics), (_, _, _): (Entity, &Position, Option<&Status>)| {})
        .add(|_: &mut Entities| {})
        // .add(|_: &mut Defer| {})
        // Emit system
        .add(|receiver: &Receiver<OnKill>| receiver.0.push(OnKill(Entity::ZERO)));
    // .schedule(world);
}

impl<'a, I: Inject<'a>, F: Fn(I)> System<'a, [I; 0]> for F {
    type State = I::State;

    fn state(world: &'a mut World) -> Option<Self::State> {
        todo!()
        // I::inject(world)
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        self(I::get(state));
    }
}

impl<'a, Q: Query<'a> + 'a, F: Fn(Q)> System<'a, [Q; 1]> for F {
    type State = <&'a Group<'a, Q> as Inject<'a>>::State;

    fn state(world: &'a mut World) -> Option<Self::State> {
        todo!()
        // <&Group<Q> as Inject>::inject(world)
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        // let inner = unsafe { world.get() };
        // let group = <&Group<Q> as Inject>::get(state);
        // for (index, state) in &group.segments {
        //     let segment = &inner.segments[*index];
        //     for i in 0..segment.get().entities.len() {
        //         self(Q::get(i, state, segment));
        //     }
        // }
    }
}

impl<'a, I: Inject<'a> + Clone, Q: Query<'a> + 'a, F: Fn(I, Q)> System<'a, [(I, Q); 2]> for F {
    type State = (I::State, <&'a Group<'a, Q> as Inject<'a>>::State);

    fn state(world: &'a mut World) -> Option<Self::State> {
        // match (I::inject(world), <&Group<Q> as Inject>::inject(world)) {
        //     (Some(inject), Some(group)) => Some((inject, group)),
        //     _ => None,
        // }
        None
    }

    fn run(&self, state: &'a mut Self::State, world: &'a World) {
        let inner = unsafe { world.get() };
        let inject = I::get(&mut state.0);
        let group = <&Group<Q> as Inject>::get(&mut state.1);
        // for (index, query) in &group.segments {
        //     let segment = &inner.segments[*index];
        //     for i in 0..segment.get().entities.len() {
        //         self(inject.clone(), Q::get(i, query, segment));
        //     }
        // }
    }
}

// impl<'a, P1, P2, S1: System<'a, P1>, S2: System<'a, P2>> System<'a, (P1, P2)> for (S1, S2) {
//     type State = (S1::State, S2::State);

//     fn state(world: &'a mut World) -> Option<Self::State> {
//         S1::state(world).zip(S2::state(world))
//     }

//     fn run(&self, state: &'a mut Self::State, world: &'a World) {
//         let (system1, system2) = self;
//         let (state1, state2) = state;
//         system1.run(state1, world);
//         system2.run(state2, world);
//     }
// }

impl<'a> System<'a, ()> for () {
    type State = ();

    fn state(_: &mut World) -> Option<Self::State> {
        Some(())
    }

    fn run(&self, _: &'a mut Self::State, _: &'a World) {}
}
