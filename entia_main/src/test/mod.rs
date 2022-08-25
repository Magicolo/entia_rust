use crate as entia;
use entia::{message::keep, system::Barrier, *};
use error::Result;

pub mod create;
pub mod depend;

#[derive(Resource, Default)]
pub struct Time(f64);
#[derive(Resource, Default)]
pub struct Physics;
#[derive(Component)]
pub struct Frozen;
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Position(f64, f64, f64);
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Velocity(f64, f64, f64);
#[derive(Message, Clone)]
pub struct OnKill(Entity);

fn inject<I: Inject>() -> Result
where
    I::Input: Default,
{
    let mut world = World::new();
    world.injector::<I>()?.run(&mut world, |_| {})?;
    Ok(())
}

#[test]
fn test() {
    fn physics(scheduler: Scheduler) -> Scheduler {
        scheduler.add(|_: ((), ())| {})
    }

    fn motion(mut query: Query<(&mut Position, &Velocity)>) {
        query.each_mut(|(position, velocity)| {
            position.0 += velocity.0;
            position.1 += velocity.1;
            position.2 += velocity.2;
        })
    }

    let mut world = World::new();
    let mut runner = world
        .scheduler()
        .pipe(physics)
        .add(Barrier)
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
            (Some(Time(12.0)), None, (), (), ()),
            |_a: &Time,
             _b: &mut Physics,
             _c: Emit<OnKill>,
             _d: Receive<OnKill>,
             _e: Query<(Entity, &mut Position, &Velocity)>| {},
        )
        .add(|_: (&Time, &Physics)| {})
        .add(|_: (&Time, &Physics)| {})
        .add(|_: (&Time, Query<Option<&Position>>)| {})
        .add(Barrier)
        .add(|(_, mut query): (&Physics, Query<&mut Velocity>)| {
            query.each_mut(|velocity| velocity.0 += 1.)
        })
        .add(
            |(time, mut queries): (
                &Time,
                (Query<&mut Position, Not<Has<Frozen>>>, Query<&mut Velocity>),
            )| {
                queries.1.each_mut(|velocity| {
                    velocity.0 += time.0;
                    velocity.1 += time.0;
                    velocity.2 += time.0;
                    queries.0.each_mut(|position| {
                        position.0 += velocity.0;
                        position.1 += velocity.1;
                        position.2 += velocity.2;
                    });
                });
                for velocity in &mut queries.1 {
                    velocity.0 += time.0;
                    velocity.1 += time.0;
                    velocity.2 += time.0;
                    for position in &mut queries.0 {
                        position.0 += velocity.0;
                        position.1 += velocity.1;
                        position.2 += velocity.2;
                    }
                }
            },
        )
        .add({
            #[derive(Resource, Default)]
            struct Private(usize);

            let mut counter = 0;
            move |resource: &mut Private| {
                resource.0 += counter;
                counter += counter;
            }
        })
        .add(motion)
        .add(|mut on_kill: Emit<_>| on_kill.one(OnKill(Entity::NULL)))
        .add(|on_kill: Receive<OnKill, keep::Last<8>>| {
            for message in on_kill {
                println!("{:?}", message.0);
            }
        })
        .add(|on_kill: Receive<OnKill>| for _ in on_kill {})
        .add(|mut on_kill: Receive<OnKill>| while let Some(_) = on_kill.next() {})
        .add(
            |query: Query<Entity, (Has<Position>, Has<Velocity>)>, mut destroy: Destroy| {
                for entity in &query {
                    destroy.one(entity, true);
                }

                query.each(|entity| destroy.one(entity, false))
            },
        )
        .add(|query: Query<Entity>, mut destroy: Destroy| {
            for entity in &query {
                destroy.one(entity, false);
            }
        })
        .schedule()
        .unwrap();

    loop {
        runner.run(&mut world).unwrap();
    }
}
