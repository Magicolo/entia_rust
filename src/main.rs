use entia::*;

fn main() {
    #[derive(Default, Clone)]
    struct Time;
    #[derive(Default, Clone)]
    struct Position(Vec<usize>);
    #[derive(Default, Clone)]
    struct Frozen;
    struct Target(Entity);

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

    fn simple() -> impl StaticTemplate<Input = impl Default, State = impl Send> {
        (
            Add::new(Frozen),
            Add::new(Position(Vec::new())),
            With::new(|_| Add::new(Frozen)),
        )
    }

    fn complex() -> impl Template<Input = impl Default, State = impl Send> {
        (Spawn::new(simple()), simple(), With::new(|_| simple()))
    }

    fn dynamic(count: usize) -> impl Template<Input = impl Default, State = impl Send> {
        vec![Spawn::new(Add::new(Frozen)); count]
    }

    let mut world = World::new();
    world
        .scheduler()
        .add(|mut create: Create<_>| {
            let families = create.all((3..=4).map(dynamic));
            println!("CREATE: {:?}", families);
        })
        .schedule()
        .unwrap()
        .run(&mut world)
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
        .add(|mut create: Create<_>| {
            create.one(complex());
        })
        .add(|mut create: Create<_>| {
            create.one((
                vec![With::new(|family| {
                    Spawn::new(Add::new(Target(family.entity())))
                })],
                With::new(|family| vec![Spawn::new(Add::new(Target(family.entity())))]),
                [Spawn::new(With::new(|family| {
                    Add::new(Target(family.entity()))
                }))],
            ));
        })
        .add(
            |roots: Query<Family, Has<Target>>, children: Query<&Position>, a: Query<&Entity>| {
                for family in &roots {
                    if let Some(child) =
                        family.descend(|descendant| children.get(descendant), |_| None)
                    {
                    }
                }
                for child in roots
                    .into_iter()
                    .flat_map(|family| family.children())
                    .filter_map(|child| children.get(child))
                {}
                println!("C: {:?}", roots.len())
            },
        )
        .add(|query: Query<Entity>, mut destroy: Destroy| query.each(|entity| destroy.one(entity)))
        .schedule()
        .unwrap();

    for _ in 0..10_000_000 {
        runner.run(&mut world).unwrap();
    }
}
