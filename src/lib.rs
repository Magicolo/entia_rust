pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod duplicate;
pub mod entities;
pub mod entity;
pub mod error;
pub mod families;
pub mod family;
pub mod ignore;
pub mod inject;
pub mod local;
pub mod message;
pub mod query;
pub mod read;
pub mod system;
pub mod template;
pub mod world;
pub mod write;

pub mod core {
    pub use entia_core::*;
}

pub use crate::{
    create::Create,
    defer::Defer,
    destroy::Destroy,
    duplicate::Duplicate,
    entity::Entity,
    error::Error,
    families::Families,
    family::{
        item::{child::Child, parent::Parent},
        Family,
    },
    ignore::{Ignore, Scope},
    inject::{Inject, Injector},
    message::emit::Emit,
    message::receive::Receive,
    query::filter::{Filter, Has, Not},
    query::Query,
    system::{runner::Runner, schedule::Scheduler, IntoSystem, System},
    template::{Add, Spawn, Template, With},
    world::World,
};
pub use entia_derive::{Depend, Filter, Template};

pub type Result<T = ()> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Default)]
    struct Time(f64);
    #[derive(Default)]
    struct Physics;
    struct Frozen;
    struct Position(f64, f64, f64);
    struct Velocity(f64, f64, f64);
    #[derive(Clone)]
    struct OnKill(Entity);

    #[test]
    fn test() {
        fn physics(scheduler: Scheduler) -> Scheduler {
            scheduler.add(|_: ((), ())| {})
        }

        fn motion(group: Query<(&mut Position, &Velocity)>) {
            group.each(|(position, velocity)| {
                position.0 += velocity.0;
                position.1 += velocity.1;
                position.2 += velocity.2;
            })
        }

        let mut world = World::new();
        let mut runner = world
            .scheduler()
            .pipe(physics)
            .barrier()
            .add(|_: ()| {})
            .add(|_: &Time| {})
            .add(|_: (&Time,)| {})
            .add(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .add(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .add(|_: (&Time, &Physics, &mut Time, &mut Physics)| {})
            .add(|_: (&Time, &Physics)| {})
            .add(|group: Query<Entity>| for _ in &group {})
            .add(
                |(group,): (Query<(Entity, &mut Position)>,)| {
                    for _ in &group {}
                },
            )
            .add(
                |_: Query<
                    (Entity,),
                    (
                        Not<(Not<(Has<Frozen>, Has<Frozen>)>, Has<Frozen>)>,
                        (Has<Position>, Not<Has<Frozen>>),
                    ),
                >| {},
            )
            .add(|_: Query<(Entity, (&Position, &Velocity))>| {})
            .add(|query: Query<(&mut Position, &mut Position)>| {
                query
                    .into_iter()
                    .filter(|(position, _)| position.0 < 1.)
                    .for_each(|(_1, _2)| {});
                query.each(|_12| {});
                for _12 in &query {}
                for (_1, _2) in &query {}
            })
            // .add(|_: &'static Time| {})
            .add_with(
                (Some(Time(12.0)), None, (), 8, ()),
                |_a: &Time,
                 _b: &mut Physics,
                 _c: Emit<OnKill>,
                 _d: Receive<OnKill>,
                 _e: Query<(Entity, &mut Position, &Velocity)>| {},
            )
            .add(|_: (&Time, &Physics)| {})
            .add(|_: (&Time, &Physics)| {})
            .add(|_: (&Time, Query<Option<&Position>>)| {})
            .barrier()
            .add(|(_, query): (&Physics, Query<&mut Velocity>)| {
                query.each(|velocity| velocity.0 += 1.)
            })
            .add(
                |(time, queries): (
                    &Time,
                    (Query<&mut Position, Not<Has<Frozen>>>, Query<&mut Velocity>),
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
            .add({
                #[derive(Default)]
                struct Private(usize);

                let mut counter = 0;
                move |resource: &mut Private| {
                    resource.0 += counter;
                    counter += counter;
                }
            })
            .add(motion)
            .add(|mut on_kill: Emit<_>| on_kill.emit(OnKill(Entity::default())))
            .add_with((8,), |on_kill: Receive<OnKill>| {
                for message in on_kill {
                    println!("{:?}", message.0);
                }
                ""
            })
            .add(|on_kill: Receive<OnKill>| for _ in on_kill {})
            .add(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
            .add(
                |query: Query<Entity, (Has<Position>, Has<Velocity>)>, mut destroy: Destroy| {
                    for entity in &query {
                        destroy.one(entity);
                    }

                    query.each(|entity| destroy.one(entity))
                },
            )
            .add(|query: Query<Entity>, mut destroy: Destroy| {
                for entity in &query {
                    destroy.one(entity);
                }
            })
            .schedule()
            .unwrap();

        loop {
            runner.run(&mut world).unwrap();
        }
    }
}
