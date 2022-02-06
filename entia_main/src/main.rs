use entia::*;
use entia_main as entia;

fn main() {
    #[derive(Resource, Default, Clone)]
    struct Time;
    #[derive(Resource, Default, Clone)]
    struct Physics;
    #[derive(Component, Default, Clone)]
    struct Position(Vec<usize>);
    #[derive(Component, Default, Clone)]
    struct Velocity(Vec<usize>);
    #[derive(Component, Default, Clone)]
    struct Frozen;
    #[derive(Component)]
    struct Target(Entity);
    #[derive(Message, Default, Clone)]
    struct OnKill;
    #[derive(Message, Default, Clone)]
    struct OnDeath(Entity);
    #[derive(Component)]
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
            |roots: Query<Family, Has<Target>>,
             children: Query<&Position>,
             _query: Query<Entity>| {
                for family in &roots {
                    if let Some(_child) =
                        family.descend(|descendant| children.get(descendant), |_| None)
                    {
                    }
                }
                for _child in roots
                    .into_iter()
                    .flat_map(|family| family.children())
                    .filter_map(|child| children.get(child))
                {}
                // println!("C: {:?}", roots.len())
            },
        )
        .add(|query: Query<Entity>, mut destroy: Destroy| query.each(|entity| destroy.one(entity)))
        .add(|mut receive: Receive<OnDeath>, mut destroy: Destroy| destroy.all(&mut receive))
        .add(|mut query: Query<Entity, Has<Dead>>, mut destroy: Destroy| destroy.all(&mut query))
        .add(
            |mut receive: Receive<OnDeath>| {
                if let Some(_message) = receive.first() {}
            },
        )
        .add(|mut emit: Emit<_>| emit.one(OnDeath(Entity::NULL)))
        .schedule()
        .unwrap();

    for _ in 0..10_000_000 {
        runner.run(&mut world).unwrap();
    }
}
