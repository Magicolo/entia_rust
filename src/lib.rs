pub mod call;
pub mod change;
mod component;
mod defer;
mod entity;
mod group;
pub mod inject;
pub mod message;
pub mod query;
mod resource;
pub mod system;
pub mod world;

pub use call::*;
pub use component::Component;
pub use defer::Defer;
pub use entity::Entity;
pub use group::Group;
pub use inject::Inject;
pub use message::Message;
pub use query::{And, Not, Query};
pub use resource::Resource;
pub use system::{Runner, Scheduler};
pub use world::{Template, World};

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
        struct Time(f64);
        struct Physics;
        struct Position(f64, f64, f64);
        struct Velocity(f64, f64, f64);
        impl Resource for Time {}
        impl Resource for Physics {}
        impl Component for Position {}
        impl Component for Velocity {}

        fn physics(scheduler: Scheduler) -> Scheduler {
            scheduler.system(|_: ((), ())| 1)
        }

        fn ui(scheduler: Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

        // fn motion(group: Group<(&mut Position, &Velocity)>) {
        //     group.each(|(position, velocity)| {
        //         position.0 += velocity.0;
        //         position.1 += velocity.1;
        //         position.2 += velocity.2;
        //     });
        // }

        let world = World::new();
        let mut runner = world
            .scheduler()
            .pipe(physics)
            .pipe(ui)
            .synchronize()
            .system(|_: ()| {})
            .system(|_: &Time| {})
            .system(|_: (&Time,)| {})
            .system(|_: &Time, _: &Physics, _: &mut Time, _: &mut Physics| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|_: Group<Entity>| {})
            // .system(|(group,): (Group<(Entity, &mut Position)>,)| {
            //     for _ in &group {}
            //     for _ in group {}
            // })
            .system(|_: Group<(Entity, And<&Position>)>| {})
            .system(|_: Group<(Entity, (&Position, &Velocity))>| {})
            // .system(|group: Group<(&mut Position, &mut Position)>| {
            //     group.each(|(_1, _2)| {});
            //     group.each(|_12| {});
            //     for _12 in &group {}
            //     for (_1, _2) in group {}
            // })
            // .system(|_: &'static Time| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|_: (&Time, Group<Option<&Position>>)| {})
            .synchronize()
            .system(|_: (&Physics, Group<&Velocity>)| {})
            // .system(
            //     |(time, groups): (&Time, (Group<&mut Position>, Group<&mut Velocity>))| {
            //         groups.1.each(|velocity| {
            //             velocity.0 += time.0;
            //             velocity.1 += time.0;
            //             velocity.2 += time.0;
            //             groups.0.each(|position| {
            //                 position.0 += velocity.0;
            //                 position.1 += velocity.1;
            //                 position.2 += velocity.2;
            //             });
            //         });
            //         for velocity in &groups.1 {
            //             velocity.0 += time.0;
            //             velocity.1 += time.0;
            //             velocity.2 += time.0;
            //             for position in &groups.0 {
            //                 position.0 += velocity.0;
            //                 position.1 += velocity.1;
            //                 position.2 += velocity.2;
            //             }
            //         }
            //     },
            // )
            // .system({
            //     struct Private;
            //     impl Resource for Private {}
            //     |_: (Defer,), mut state: State<usize>, _: &Private| {
            //         *state += 1;
            //     }
            // })
            // .system(motion)
            .schedule()
            .unwrap();

        loop {
            runner.run();
        }
    }
}
