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
pub mod query;
pub mod resource;
pub mod system;
pub mod world;

pub use call::Call;
pub use component::Component;
pub use defer::Defer;
pub use entity::{Entities, Entity};
pub use inject::Inject;
pub use item::{And, Item, Not};
pub use local::Local;
pub use message::{Emit, Message, Receive};
pub use query::Query;
pub use resource::Resource;
pub use system::{Runner, Scheduler};
pub use world::{Template, World};

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
    ($m:ident, $a:ident, $b:ident) => {
        $m!($a, $b);
        crate::recurse!($m);
    };
    ($m:ident, $a:ident, $b:ident, $($as:ident, $bs:ident),*) => {
        $m!($a, $b, $($as, $bs),*);
        crate::recurse!($m, $($as, $bs),*);
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
            scheduler.system(|_: ((), ())| {})
        }

        fn ui(scheduler: Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

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
            .pipe(physics)
            .pipe(ui)
            .synchronize()
            .system(|_: ()| {})
            .system(|_: &Time| {})
            .system(|_: (&Time,)| {})
            .system(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .system(|_: (&Time, &Physics, &mut Time, &mut Physics)| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|group: Query<Entity>| for _ in &group {})
            .system(
                |(group,): (Query<(Entity, &mut Position)>,)| {
                    for _ in &group {}
                },
            )
            .system(|_: Query<(Entity, Not<&Frozen>, And<&Position>)>| {})
            .system(|_: Query<(Entity, (&Position, &Velocity))>| {})
            .system(|group: Query<(&mut Position, &mut Position)>| {
                group.each(|(_1, _2)| {});
                group.each(|_12| {});
                for _12 in &group {}
                for (_1, _2) in &group {}
            })
            // .system(|_: &'static Time| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|_: (&Time, Query<Option<&Position>>)| {})
            .synchronize()
            .system(|_: (&Physics, Query<&Velocity>)| {})
            .system(
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
            .system({
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
            .system(motion)
            .system(|mut on_kill: Emit<OnKill>| on_kill.emit(OnKill(Entity::default())))
            .system(|on_kill: Receive<OnKill>| for _ in on_kill {})
            .system(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
            .system(|_: &Entities| {})
            .system(|_: &mut Entities| {})
            .schedule()
            .unwrap();

        loop {
            runner.run(&mut world);
        }
    }
}
