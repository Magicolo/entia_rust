use entia::{entities::Entities, message::keep, system::Barrier, *};
use entia_main as entia;

fn main() {
    #[derive(Default, Clone)]
    struct Time;
    #[derive(Default, Clone)]
    struct Physics;
    #[derive(Default, Clone)]
    struct Position(Vec<usize>);
    #[derive(Default, Clone)]
    struct Velocity(Vec<usize>);
    #[derive(Default, Copy, Clone)]
    struct Frozen;
    struct Target(Entity);
    #[derive(Default, Clone)]
    struct OnKill;
    #[derive(Default, Clone)]
    struct OnDeath(Entity);
    struct Dead;

    impl Into<Entity> for OnDeath {
        fn into(self) -> Entity {
            self.0
        }
    }

    let create = || {
        let mut counter = 0;
        move |mut create: Create<_>| {
            let position = Position(Vec::with_capacity(counter));
            counter += counter / 100 + 1;
            create.one((Add::new(position.clone()), Add::new(Frozen)));
            create.all((0..counter).map(|_| (Add::new(position.clone()), Add::new(Frozen))));
            create.defaults(counter);
            create.clones(counter, (Add::new(position), Add::new(Frozen)));
        }
    };

    fn simple() -> impl StaticTemplate {
        (
            Add::new(Frozen),
            Add::new(Position(Vec::new())),
            With::new(|_| Add::new(Frozen)),
        )
    }

    fn complex() -> impl Template {
        (Spawn::new(simple()), simple(), With::new(|_| simple()))
    }

    fn dynamic(count: usize) -> impl Template {
        vec![Spawn::new(Add::new(Frozen)); count]
    }

    let mut world = World::new();
    world
        .run(|mut create: Create<_>| {
            let families = create.all((3..=4).map(dynamic));
            println!("CREATE: {:?}", families);
        })
        .unwrap();

    let _ = world
        .scheduler()
        // .add(|mut create: Create<_>| {
        //     create.one(
        //         ().add(Position(vec![]))
        //             .add(Velocity(vec![]))
        //             .spawn(
        //                 ().add(Frozen)
        //                     .add(Frozen)
        //                     .add(Frozen)
        //                     .add(Frozen)
        //                     .spawn(())
        //                     .add(Frozen),
        //             )
        //             .spawn(())
        //             .spawn(())
        //             .spawn(()),
        //     );
        // })
        .add(|_query: Query<Entity>, mut _adopt: Adopt| {})
        .add(|| {})
        .schedule()
        .unwrap();
    let mut runner = world
        .scheduler()
        .add(create())
        .add(create())
        .add(create())
        .add(create())
        .add(create())
        .add(create())
        .add(create())
        .add(create())
        .add(|mut create: Create<_>| {
            create.one(());
        })
        .add(|mut create: Create<_>| {
            create.one((
                Add::new(Frozen),
                Add::new(Frozen),
                Add::new(Frozen),
                Add::new(Frozen),
                Add::new(Frozen),
                Add::new(Frozen),
            ));
        })
        .add(Barrier)
        // .add(Boba::new::<Read<Position>, _, _>(|physics: &Physics| {
        //     |position: &Position| {}
        // }))
        .add(|mut create: Create<_>| {
            create.one(complex());
        })
        .add(|mut create: Create<_>| {
            create.one((
                vec![With::new(|family| {
                    Spawn::new(Add::new(Target(family.entity())))
                })],
                With::new(|family| [Spawn::new(Add::new(Target(family.entity())))]),
                [Spawn::new(With::new(|family| {
                    Add::new(Target(family.entity()))
                }))],
            ));
        })
        .add(
            |roots: Query<Family, Has<Target>>,
             children: Query<&Position>,
             _query: Query<Entity>| {
                for family in &roots {
                    // 'Err' interrupts the descent.
                    if let Err(_child) = family.try_descend(
                        (),
                        |descendant, _| children.get(descendant).map_or(Ok(()), Err),
                        |_, _| Ok(()),
                    ) {}
                }
                for _child in roots
                    .into_iter()
                    .flat_map(|family| family.children())
                    .filter_map(|child| children.get(child))
                {}
                // println!("C: {:?}", roots.len())
            },
        )
        .add(|query: Query<Entity>, mut destroy: Destroy| {
            query.each(|entity| destroy.one(entity, true))
        })
        .add(
            |mut query: Query<(&mut Position, &Velocity, Option<&mut Frozen>)>| {
                for (_positions, _velocities, _frozen) in query.chunks() {}
                for (positions, velocities, frozen) in query.chunks_mut() {
                    match frozen {
                        Some(frozen) => frozen[0] = Frozen,
                        None => positions[0].0[0] += velocities[0].0[0],
                    }
                }
            },
        )
        .add(|entities: &Entities| {
            for root in entities.roots() {
                let root = entities.root(root);
                if let Some(_) = entities.parent(root) {
                    panic!("")
                } else if entities.has(root) {
                } else {
                    panic!("")
                }
                let mut down = 0;
                let mut up = 0;
                entities.descend(
                    root,
                    |child| down += child.index(),
                    |child| up += child.index(),
                );
            }
        })
        // Strangely valid things...
        .add(|_: Query<&usize>, _: &mut bool, _: Receive<()>, _: Emit<Entity>| {})
        .add(
            |receive: Receive<OnDeath, keep::First<10>>, mut destroy: Destroy| {
                destroy.all(receive, true)
            },
        )
        .add(|query: Query<Entity, Has<Dead>>, mut destroy: Destroy| {
            destroy.all(query.iter(), true)
        })
        .add(
            |mut receive: Receive<OnDeath, keep::Last<5>>| {
                if let Some(_message) = receive.next() {}
            },
        )
        .add(|mut emit: Emit<_>| emit.one(OnDeath(Entity::NULL)))
        .schedule()
        .unwrap();

    for _ in 0..10_000_000 {
        runner.run(&mut world).unwrap();
    }
}
