use entia::{
    initial::{spawn, with, Initial, StaticInitial},
    prelude::*,
};

/*
Coherence rules:
- Within one system, structural operations will not be visible until resolution.
For example, it is ok for any query to overlap with a 'Create' operation, but to ensure coherence,
the newly created entities will not be added to the query until the system has completed its execution.
For example, it is also ok for many '
- Between systems, structural operations will only force an ordering if an operation could require the result of a
previous one.
For example, if system A has 'Add' and system B has 'Remove', they will be ordered by the system declaration order
since after the execution of both systems, the presence or not of components that the 'Add/Remove' modify must be
deterministic.
- Create: Since other threads don't have access to newly created entities other than through a 'Write' dependency
(which incurs an ordering), it is compatible with other defered operation.

________| Create  | Destroy | Add     | Remove  | Read    | Write   |
Create  |    Y    |    Y    |    Y    |    Y    |    N    |    N    |
Destroy |   Y*1   |    Y    |   Y*2   |   Y*2   |    N    |    N    |
Add     |   Y*1   |    Y    |   Y*3   |    Y    |    N    |    N    |
Remove  |   Y*1   |    Y    |    Y    |    Y    |    N    |    N    |
Read    |    Y    |    Y    |    Y    |    Y    |    Y    |    N    |
Write   |    Y    |    Y    |    Y    |    Y    |    N    |    N    |

- Should read as: if the system 'A' declared first has the row operation and system 'B' has the column operation
declared second, can they be executed in parallel to ensure a deterministic outcome and to respect declaration
order intuition?
- When 'Read/Write' operations are declared first, structural changes are not expected to be observed and as such,
they don't force an ordering.
- When 'Read/Write' operations are declared second, structural changes must be resolved for overlapping segments.
*1: Coherence is satisfied by those operations only if resolution is ordered. Note that batch operations such as
'add_all/remove_all/destroy_all' can easily overlap with other operations.
*2: Coherence is satisfied by those operations since the outcome is deterministic.
*3: Coherence is satisfied as long as components are not observed.

STATIC VS DYNAMIC:
- Static allows all segments to be known at initialization time, ensuring that scheduling is most efficient and knowns all dependencies.
- Dynamic makes entities hard to debug since a removed component loses all its data.

// PROS
// - Batch operations can be implemented efficiently if a whole segment is modified.

// CONS:
// - Adds many more segments and spreads entities more.
// - Adds many deferred operations that will be hard or inefficient to parallelize.
// - Adds a lot of complexity when trying to figure out where a component comes from.
// - Probably does not add much performance when taking into account the loss in parallelization and resolution factors.
// - Probably does not add much memory efficiency and is probably worse. While segments stores *might* be smaller overall,
// chances are that a lot of entity slots will be reserved more than once.
// - Make dependency management more complex.

TODO:
- Factory implements 'Initial' but will hide the details of the components to make it easier to compose.
Note that the current 'Initial' trait will be required to allow non-static declaration of 'metas'.
The factory may also allow for some additionnal validation and optimizations.
Its API could look like this:
Injector::new()
    .factory(Template::new()
        .add(Position(Vec::new()))              -> Template<()>
        .add_default::<Frozen>()                -> Template<()>
        .add_with(|x: f64| Position(vec![x]))   -> Template<(f64,)>
        .remove::<Frozen>()                     -> Template<(f64,)>
        .child(Template::new().add(Frozen)))    -> Template<(f64,)>

- Review 'Schedule' API.

- Implement 'Drop' for segment to free the stores.

- Fix coherence when 'Create' and 'Destroy' appear in the same system or disallow those systems.
A 'Destroy::all' operation could destroy entities that have not been created yet since a later 'Create' might not need to
defer its operation. A possible solution would be for the 'Destroy::all' operation to store 'segment.reserved'.

- Similar to 'Create' and 'Destroy', move resolve logic of 'Emit' to run time when possible. As long as no resize are required,
it should be possible to do so by adding a 'reserved: AtomicUsize' to queues.

- Make deferral more explicit and extensible by enforcing the format: 'Defer<Add<Freeze>>'?

'Local' could be repurposed to represent system-wide shared data.
Note that the 'Resolver' can be shared such that the 'Resolve::State' is not uselessly duplicated.

- Find a way to definitely know which segments overlap between deferred operations and 'Read/Write'. Otherwise,
the deferred operations should have a 'Defer(Entity)' dependency on all segments.

- Allow to declare a segment as 'Static or Dynamic'. 'Static' segment contain entities that will never change their structure
while 'Dynamic' segments will allow entities to move to another segment. This would allow to allocate/deallocate batches of
static entities (such as particles) since 'Static' segments guarantee that the indices of the batch will still be valid at
deallocation time.
    - Should static entities have a different type? Otherwise, it means that a component 'add' could fail.
    - Perhaps, only the batch allocation/deallocation mechanism could use static segments?
    - Should static entities be queried differently than dynamic ones? 'Group<(Entity, And<Static>)>'?

- Find a better name for 'Modify'.
- Find a way to make 'Component', 'Resource' and 'Message' implementations exclusive.
- #[derive(Inject/Item/Modify/Filter)] macros that implements the corresponding trait for structs that hold only
fields that implement it.
- #[derive(Component/Resource/Message)] macros that implement the corresponding trait for structs.
- Clean up unnecessary #[inline].
*/

fn main() {
    #[derive(Default, Clone)]
    struct Time;
    #[derive(Default, Clone)]
    struct Position(Vec<usize>);
    #[derive(Default, Clone)]
    struct Frozen;
    struct Target(Entity);
    impl Component for Position {}
    impl Component for Frozen {}
    impl Component for Target {}
    impl Resource for Time {}

    let create = || {
        let mut counter = 0;
        move |mut create: Create<_>| {
            let position = Position(Vec::with_capacity(counter));
            counter += counter / 100 + 1;
            create.one((position.clone(), Frozen));
            create.all((0..counter).map(|_| (position.clone(), Frozen)));
            create.defaults(counter);
            create.clones((position, Frozen), counter);
        }
    };

    fn simple() -> impl StaticInitial<Input = impl Default> {
        (Frozen, Position(Vec::new()), with(|_| Frozen))
    }

    fn complex() -> impl StaticInitial<Input = impl Default> {
        (spawn(simple()), [simple()], with(|_| simple()))
    }

    fn dynamic(count: usize) -> impl Initial<Input = impl Default> {
        vec![spawn(Frozen); count]
    }

    let mut world = World::new();
    world.run(|mut create: Create<_>| {
        let families = create.all((3..=4).map(dynamic));
        println!("CREATE: {:?}", families);
    });

    let mut runner = world
        .scheduler()
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(create())
        .schedule(|mut create: Create<_>| {
            create.one(());
        })
        .schedule(|mut create: Create<_>| {
            create.one((Frozen, Frozen, Frozen, Frozen, Frozen, Frozen));
        })
        .schedule(|mut create: Create<_>| {
            create.one(complex());
        })
        .schedule(|mut create: Create<_>| {
            create.one((
                vec![with(|family| spawn(Target(family.entity())))],
                spawn(vec![with(|family| Target(family.entity()))]),
                spawn(with(|family| vec![Target(family.entity())])),
            ));
        })
        .schedule(|query: Query<Entity>| println!("C: {:?}", query.len()))
        .schedule(|mut destroy: Destroy| destroy.all())
        .runner()
        .unwrap();

    for _ in 0..10_000_000 {
        runner.run(&mut world);
    }
}
