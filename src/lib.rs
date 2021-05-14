pub mod append;
pub mod call;
pub mod change;
pub mod component;
pub mod defer;
pub mod entity;
pub mod initialize;
pub mod inject;
pub mod item;
pub mod local;
pub mod message;
pub mod prepend;
pub mod query;
pub mod read;
pub mod resource;
pub mod schedule;
pub mod system;
pub mod world;
pub mod write;

pub use call::Call;
pub use component::Component;
pub use defer::Defer;
pub use entity::{Entities, Entity};
pub use inject::{Inject, Injector};
pub use item::{And, Item, Not};
pub use local::Local;
pub use message::{Emit, Message, Receive};
pub use query::Query;
pub use resource::Resource;
pub use schedule::Scheduler;
pub use system::Runner;
pub use world::World;

/*
- Each 'World' instance could have a unique identifier (a simple counter would do) such that a runner that runs over a 'World'
can verify that it is the same instance with which it has been initialized.

- Call 'Inject::update' on instances that declared a dependency on the segment. Alternatively, add an 'on_segment_change'
method to the 'Inject' trait.

- Allow to declare a segment as 'Static or Dynamic'. 'Static' segment contain entities that will never change their structure
while 'Dynamic' segments will allow entities to move to another segment. This would allow to allocate/deallocate batches of
static entities (such as particles) since 'Static' segments guarantee that the indices of the batch will still be valid at
deallocation time.
    - Should static entities have a different type? Otherwise, it means that a component 'add' could fail.
    - Perhaps, only the batch allocation/deallocation mechanism could use static segments?
    - Should static entities be queried differently than dynamic ones? 'Group<(Entity, And<Static>)>'?
*/

/*
SYSTEMS
- Runners must be able to re-initialize and re-schedule all systems when a segment is added.
- This will happen when the 'Defer' module is resolved which occurs at the next synchronization point.
- There should not be a significant performance hit since segment addition/removal is expected to be rare and happen mainly
in the first frames of execution.

RESOURCES
- There will be 1 segment per resource such that the same segment/dependency system can be used for them.
- Resource segments should only allocate 1 store with 1 slot with the resource in it.
- Resource entities must not be query-able (could be accomplished with a simple 'bool' in segments).

DEPENDENCIES
- Design a contract API that ensures that dependencies are well defined.
- To gain access to a given resource, a user must provide a corresponding 'Contract' that is provided by a 'Contractor'.
- The 'Contractor' then stores a copy of each emitted contract to later convert them into corresponding dependencies.
- Ex: System::initialize(contractor: &mut Contractor, world: &mut World) -> Store<Time> {
    world.get_resource(contractor.resource(TypeId::of::<Time>()))
        OR
    world.get_resource::<Time>(contractor)
        OR
    contractor.resource::<Time>(world) // This doesn't require the 'World' to know about the 'Contractor'.
        OR
    contractor.resource::<Time>() // The contractor can hold its own reference to the 'World'.
}
*/

#[macro_export]
macro_rules! recurse {
    ($m:ident) => {
        $m!();
    };
    ($m:ident, $p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        $m!($p, $t $(,$ps, $ts)*);
        crate::recurse!($m $(,$ps, $ts)*);
    };
}

#[macro_export]
macro_rules! recurse_32 {
    ($m:ident) => {
        crate::recurse!(
            $m, p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9,
            p10, T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18,
            T18, p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26,
            p27, T27, p28, T28, p29, T29, p30, T30, p31, T31
        );
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        #[derive(Default)]
        struct Time(f64);
        #[derive(Default)]
        struct Physics;
        struct Frozen;
        struct Position(f64, f64, f64);
        struct Velocity(f64, f64, f64);
        #[derive(Clone)]
        struct OnKill(Entity);
        impl Resource for Time {}
        impl Resource for Physics {}
        impl Component for Frozen {}
        impl Component for Position {}
        impl Component for Velocity {}
        impl Message for OnKill {}

        fn physics(scheduler: Scheduler) -> Scheduler {
            scheduler.schedule(|_: ((), ())| {})
        }

        // fn ui(injector: Injector) -> Injector {
        //     injector.inject::<()>().schedule(|_a| {})
        // }

        fn motion(group: Query<(&mut Position, &Velocity)>) {
            group.each(|(position, velocity)| {
                position.0 += velocity.0;
                position.1 += velocity.1;
                position.2 += velocity.2;
            });
        }

        let mut world = World::new();
        let mut runner = world
            .scheduler()
            .schedule(physics)
            // .schedule(ui)
            .synchronize()
            .schedule(|_: ()| {})
            .schedule(|_: &World| {})
            .schedule(|_: &Time| {})
            .schedule(|_: (&Time,)| {})
            .schedule(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .schedule(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .schedule(|_: (&Time, &Physics, &mut Time, &mut Physics)| {})
            .schedule(|_: (&Time, &Physics)| {})
            .schedule(|group: Query<Entity>| for _ in &group {})
            .schedule(
                |(group,): (Query<(Entity, &mut Position)>,)| {
                    for _ in &group {}
                },
            )
            .schedule(|_: Query<(Entity, Not<&Frozen>, And<&Position>)>| {})
            .schedule(|_: Query<(Entity, (&Position, &Velocity))>| {})
            .schedule(|query: Query<(&mut Position, &mut Position)>| {
                query.each(|(_1, _2)| {});
                query.each(|_12| {});
                for _12 in &query {}
                for (_1, _2) in &query {}
            })
            // .schedule(|_: &'static Time| {})
            // .schedule(|_: &mut World| {
            //     let mut counter = 1.;
            //     move |time: &Time, _: &World| {
            //         counter += time.0 * counter;
            //     }
            // })
            .injector()
            .inject::<&Time>()
            .inject::<&mut Physics>()
            .inject::<Emit<OnKill>>()
            .inject_with::<Receive<OnKill>>(8)
            .inject::<Query<(Entity, &mut Position, &Velocity)>>()
            .schedule(|_a, _b, _c, _d, _e| {})
            //
            .schedule(|_: (&Time, &Physics)| {})
            .schedule(|_: (&Time, &Physics)| {})
            .schedule(|_: (&Time, Query<Option<&Position>>)| {})
            .synchronize()
            .schedule(|(_, query): (&Physics, Query<&mut Velocity>)| {
                query.each(|velocity| velocity.0 += 1.)
            })
            .schedule(
                |(time, groups): (&Time, (Query<&mut Position>, Query<&mut Velocity>))| {
                    groups.1.each(|velocity| {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;
                        groups.0.each(|position| {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        });
                    });
                    for velocity in &groups.1 {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;
                        for position in &groups.0 {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        }
                    }
                },
            )
            .schedule({
                #[derive(Default)]
                struct Private(usize);
                impl Resource for Private {}

                let mut counter = 0;
                move |mut state: Local<usize>, resource: &mut Private| {
                    *state += counter;
                    resource.0 += counter;
                    counter += counter;
                }
            })
            .schedule(motion)
            .schedule(|mut on_kill: Emit<OnKill>| on_kill.emit(OnKill(Entity::default())))
            .schedule(((8,), |on_kill: Receive<OnKill>| for _ in on_kill {}))
            .schedule(|on_kill: Receive<OnKill>| for _ in on_kill {})
            .schedule(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
            .schedule(|_: Entities| {})
            .runner()
            .unwrap();

        loop {
            runner.run(&mut world);
        }
    }
}
