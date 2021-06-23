pub mod component;
pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod emit;
mod entities;
pub mod entity;
pub mod filter;
pub mod initial;
pub mod inject;
pub mod item;
mod local;
pub mod message;
pub mod query;
pub mod read;
pub mod receive;
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
    pub use crate::component::Component;
    pub use crate::create::Create;
    pub use crate::destroy::Destroy;
    pub use crate::emit::Emit;
    pub use crate::entity::Entity;
    pub use crate::filter::Not;
    pub use crate::inject::Injector;
    pub use crate::message::Message;
    pub use crate::query::Query;
    pub use crate::receive::Receive;
    pub use crate::resource::Resource;
    pub use crate::schedule::Scheduler;
    pub use crate::system::Runner;
    pub use crate::world::World;
}

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
    fn test() {
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
            .schedule_with(
                Injector::new()
                    .inject::<&Time>()
                    .inject::<&mut Physics>()
                    .inject::<Emit<OnKill>>()
                    .inject_with::<Receive<OnKill>>(8)
                    .inject::<Query<(Entity, &mut Position, &Velocity)>>(),
                |_a, _b, _c, _d, _e| {},
            )
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
                move |resource: &mut Private| {
                    resource.0 += counter;
                    counter += counter;
                }
            })
            .schedule(motion)
            .schedule(|mut on_kill: Emit<OnKill>| on_kill.emit(OnKill(Entity::default())))
            .schedule(((8,), |on_kill: Receive<OnKill>| for _ in on_kill {}))
            .schedule(|on_kill: Receive<OnKill>| for _ in on_kill {})
            .schedule(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
            // .schedule(
            //     |query: Query<Entity>, mut add: Add<(Position, Option<Velocity>)>| {
            //         for entity in &query {
            //             add.add(entity, (Position(1., 2., 3.), None));
            //             add.add(entity, (Position(1., 2., 3.), Some(Velocity(3., 2., 1.))));
            //         }
            //     },
            // )
            // .schedule(
            //     |mut create: Create<(Position, Option<Velocity>)>, mut add: Add<Frozen>| {
            //         let _ = create.one((Position(1., 2., 3.), None));
            //         let entity = create.one((Position(1., 2., 3.), Some(Velocity(3., 2., 1.))));
            //         add.add(entity, Frozen);
            //     },
            // )
            // .schedule(
            //     |query: Query<Entity>, mut remove: Remove<(Position, Option<Velocity>)>| {
            //         for entity in &query {
            //             remove.remove(entity);
            //         }
            //     },
            // )
            // Removes the 'Position' component of all entities that don't have a 'Player' component and that have a 'Frozen' component.
            // .schedule(|mut remove: Remove<Position, (Not<Player>, Frozen)>| remove.remove_all())
            .schedule(
                |query: Query<Entity>, mut destroy: Destroy<(), (Position, Velocity)>| {
                    for entity in &query {
                        destroy.one(entity);
                    }
                },
            )
            .schedule(|query: Query<Entity>, mut destroy: Destroy| {
                for entity in &query {
                    destroy.one(entity);
                }
            })
            .runner()
            .unwrap();

        loop {
            runner.run(&mut world);
        }
    }
}
