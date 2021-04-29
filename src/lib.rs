pub mod call;
pub mod change;
pub mod inject;
pub mod query;
pub mod system;
pub mod world;

pub use call::*;
pub use inject::{Defer, Group, Inject};
pub use query::{And, Not, Query};
pub use system::{Runner, Scheduler, System};
pub use world::{Component, Entity, Resource, Template, World};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        struct Time(f64);
        struct Physics;
        struct Position(f64, f64, f64);
        struct Velocity(f64, f64, f64);
        impl Resource for Time {}
        impl Resource for Physics {}
        impl Component for Position {}
        impl Component for Velocity {}

        fn physics(scheduler: &Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

        fn ui(scheduler: &Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

        let scheduler = Scheduler::default()
            .pipe(physics)
            .pipe(ui)
            .synchronize()
            .system(|_: &Time| {})
            .system(|_: (&Time,)| {})
            .system(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|_: Group<Entity>| {})
            .system(|(group,): (Group<(Entity, &mut Position)>,)| {
                for _ in &group {}
                for _ in group {}
            })
            .system(|_: Group<(Entity, And<&Position>)>| {})
            .system(|_: Group<(Entity, (&Position, &Velocity))>| {})
            // Must be prevented since it breaks the invariants of Rust.
            // - will be allowed at compile-time, but will fail to initialize
            .system(|group: Group<(&mut Position, &mut Position)>| {
                group.each(|(_1, _2)| {});
                group.each(|_12| {});
                for _12 in &group {}
                for (_1, _2) in group {}
            })
            .system(|_: (&Time, &Physics)| {})
            .system(|_: (&Time, Group<Option<&Position>>)| {})
            .synchronize()
            .system(|_: (&Physics, Group<&Velocity>)| {})
            .system(
                |(time, (group1, group2)): (
                    &Time,
                    (Group<&mut Position>, Group<&mut Velocity>),
                )| {
                    group2.each(|velocity| {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;

                        group1.each(|position| {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        });
                    });

                    for velocity in &group2 {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;

                        for position in &group1 {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        }
                    }
                },
            )
            .system(|_: (Defer,)| {});

        let world = World::new();
        let mut runner = scheduler.schedule(&world).unwrap();
        loop {
            runner.run();
        }
    }
}
