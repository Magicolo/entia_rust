use entia::prelude::*;

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

TODO:
- Deferred operations must share the same defer queue but each system should have its own defer queue. This way,
resolution order will be preserved between systems and no thread safety is needed.
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
    let create = || {
        let mut counter = 0;
        move |mut create: Create<()>| {
            counter += counter / 1000 + 1;
            create.create(());
            create.create_default(counter);
        }
    };

    let mut world = World::new();
    let mut runner = world
        .scheduler()
        .schedule(create())
        .schedule(|query: Query<Entity>| println!("C: {:?}", query.len()))
        .schedule(create())
        .schedule(|mut destroy: Destroy| destroy.destroy_all())
        .runner()
        .unwrap();

    for _ in 0..1000000 {
        runner.run(&mut world);
    }
}
