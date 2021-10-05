mod component;
mod create;
mod defer;
mod depend;
mod destroy;
mod emit;
mod entities;
mod entity;
mod family;
mod filter;
mod initial;
mod inject;
mod item;
mod local;
mod message;
mod query;
mod read;
mod receive;
mod resource;
mod schedule;
mod segment;
mod system;
mod world;
mod write;

pub mod core {
    pub use entia_core::*;
}

pub use crate::{
    component::Component,
    create::Create,
    defer::Defer,
    destroy::Destroy,
    emit::Emit,
    entities::Direction,
    entity::Entity,
    family::initial::{Families, Family},
    family::item::{Child, Parent},
    filter::Not,
    initial::{spawn, with, Initial, StaticInitial},
    inject::Injector,
    message::Message,
    query::Query,
    receive::Receive,
    resource::Resource,
    schedule::Scheduler,
    system::Runner,
    world::World,
};

#[cfg(test)]
mod test {
    use super::*;

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
            .pipe(physics)
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
                (Some(Time(12.0)), None, (), 8, ()),
                |_a: &Time,
                 _b: &mut Physics,
                 _c: Emit<OnKill>,
                 _d: Receive<OnKill>,
                 _e: Query<(Entity, &mut Position, &Velocity)>| {},
            )
            .schedule(|_: (&Time, &Physics)| {})
            .schedule(|_: (&Time, &Physics)| {})
            .schedule(|_: (&Time, Query<Option<&Position>>)| {})
            .synchronize()
            .schedule(|(_, query): (&Physics, Query<&mut Velocity>)| {
                query.each(|velocity| velocity.0 += 1.)
            })
            .schedule(
                |(time, queries): (
                    &Time,
                    (Query<&mut Position, Not<Frozen>>, Query<&mut Velocity>),
                )| {
                    queries.1.each(|velocity| {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;
                        queries.0.each(|position| {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        });
                    });
                    for velocity in &queries.1 {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;
                        for position in &queries.0 {
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
            .schedule(|mut on_kill: Emit<_>| on_kill.emit(OnKill(Entity::default())))
            .schedule_with((8,), |on_kill: Receive<OnKill>| {
                for message in on_kill {
                    println!("{:?}", message.0);
                }
            })
            .schedule(|on_kill: Receive<OnKill>| for _ in on_kill {})
            .schedule(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
            .schedule(
                |query: Query<Entity, (Position, Velocity)>, mut destroy: Destroy| {
                    for entity in &query {
                        destroy.one(entity);
                    }

                    destroy.all(&query);
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
