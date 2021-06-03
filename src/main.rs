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
*/

fn main() {
    let mut world = World::new();
    let mut runner = world
        .scheduler()
        .schedule(|mut create: Create<()>| {
            println!("A");
            let entity = create.create(());
            println!("A1: {:?}", entity);
            let entities = create.create_default(100);
            println!("A2: {:?}", entities);
        })
        .synchronize()
        .schedule(|query: Query<Entity>| {
            println!("B");
            query.each(|entity: Entity| println!("B1: {:?}", entity));
            query.each(|entity: Entity| println!("B2: {:?}", entity));
        })
        .synchronize()
        .schedule(|mut destroy: Destroy| {
            println!("C");
            destroy.destroy_all();
        })
        .synchronize()
        .schedule(|query: Query<Entity>| {
            println!("D");
            query.each(|entity: Entity| println!("D1: {:?}", entity));
            query.each(|entity: Entity| println!("D2: {:?}", entity));
        })
        .runner()
        .unwrap();
    runner.run(&mut world);
}
