pub mod add;
pub mod component;
pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod emit;
pub mod entities;
pub mod entity;
pub mod filter;
pub mod inject;
pub mod item;
pub mod local;
pub mod message;
pub mod modify;
pub mod not;
pub mod query;
pub mod read;
pub mod receive;
pub mod remove;
pub mod resource;
pub mod schedule;
pub mod segment;
pub mod system;
pub mod world;
pub mod write;

pub mod core {
    pub use entia_core::utility;
    pub use entia_core::*;
}

pub mod prelude {
    pub use crate::add::Add;
    pub use crate::component::Component;
    pub use crate::create::Create;
    pub use crate::destroy::Destroy;
    pub use crate::emit::Emit;
    pub use crate::entity::Entity;
    pub use crate::inject::Injector;
    pub use crate::local::Local;
    pub use crate::message::Message;
    pub use crate::not::Not;
    pub use crate::query::Query;
    pub use crate::receive::Receive;
    pub use crate::remove::Remove;
    pub use crate::resource::Resource;
    pub use crate::schedule::Scheduler;
    pub use crate::system::Runner;
    pub use crate::world::World;
}

/*
- Call 'Inject::update' on instances that declared a dependency on the segment. Alternatively, add an 'on_segment_change'
method to the 'Inject' trait.

- Allow to declare a segment as 'Static or Dynamic'. 'Static' segment contain entities that will never change their structure
while 'Dynamic' segments will allow entities to move to another segment. This would allow to allocate/deallocate batches of
static entities (such as particles) since 'Static' segments guarantee that the indices of the batch will still be valid at
deallocation time.
    - Should static entities have a different type? Otherwise, it means that a component 'add' could fail.
    - Perhaps, only the batch allocation/deallocation mechanism could use static segments?
    - Should static entities be queried differently than dynamic ones? 'Group<(Entity, And<Static>)>'?

- #[derive(Inject)] macro that implements 'Inject' for structs that hold only fields that implement 'Inject'.
- #[derive(Item)] macro that implements 'Item' for structs that hold only fields that implement 'Item'.

- Keep blanket implementations for 'Component/Resource/Message'?
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

#[cfg(test)]
mod test {
    use super::prelude::*;

    #[derive(Default)]
    struct Time(f64);
    #[derive(Default)]
    struct Physics;
    struct Player;
    struct Enemy;
    struct Frozen;
    struct Position(f64, f64, f64);
    struct Velocity(f64, f64, f64);
    #[derive(Clone)]
    struct OnKill(Entity);
    impl Resource for Time {}
    impl Resource for Physics {}
    impl Component for Player {}
    impl Component for Enemy {}
    impl Component for Position {}
    impl Component for Velocity {}
    impl Component for Frozen {}
    impl Message for OnKill {}

    #[test]
    fn create_entity() {
        let mut world = World::new();
        let mut runner = world
            .scheduler()
            .schedule(|mut create: Create<()>| {
                let entity = create.create(());
                println!("{:?}", entity);
            })
            .runner()
            .unwrap();
        runner.run(&mut world);
    }

    // #[test]
    fn _test() {
        fn physics(scheduler: Scheduler) -> Scheduler {
            scheduler.schedule(|_: ((), ())| {})
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
            .schedule(
                |_: Query<
                    (Entity,),
                    (
                        Not<(Not<(Frozen, Frozen)>, Frozen)>,
                        (Position, Not<Frozen>),
                    ),
                >| {},
            )
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
            .schedule(
                |query: Query<Entity>, mut add: Add<(Position, Option<Velocity>)>| {
                    for entity in &query {
                        add.add(entity, (Position(1., 2., 3.), None));
                        add.add(entity, (Position(1., 2., 3.), Some(Velocity(3., 2., 1.))));
                    }
                },
            )
            .schedule(
                |mut create: Create<(Position, Option<Velocity>)>, mut add: Add<Frozen>| {
                    let _ = create.create((Position(1., 2., 3.), None));
                    let entity = create.create((Position(1., 2., 3.), Some(Velocity(3., 2., 1.))));
                    add.add(entity, Frozen);
                },
            )
            .schedule(
                |query: Query<Entity>, mut remove: Remove<(Position, Option<Velocity>)>| {
                    for entity in &query {
                        remove.remove(entity);
                    }
                },
            )
            // Removes the 'Position' component of all entities that don't have a 'Player' component and that have a 'Frozen' component.
            .schedule(|mut remove: Remove<Position, (Not<Player>, Frozen)>| remove.remove_all())
            .schedule(
                |query: Query<Entity>, mut destroy: Destroy<(Position, Velocity)>| {
                    for entity in &query {
                        destroy.destroy(entity);
                    }
                },
            )
            .schedule(|query: Query<Entity>, mut destroy: Destroy| {
                for entity in &query {
                    destroy.destroy(entity);
                }
            })
            .runner()
            .unwrap();

        loop {
            runner.run(&mut world);
        }
    }
}
