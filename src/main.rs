use entia::prelude::*;

fn main() {
    let mut world = World::new();
    let mut runner = world
        .scheduler()
        .schedule(|mut create: Create<()>| {
            let entity = create.create(());
            println!("A: {:?}", entity);
        })
        .synchronize()
        .schedule(|query: Query<Entity>| {
            query.each(|entity: Entity| println!("B: {:?}", entity));
            query.each(|entity: Entity| println!("C: {:?}", entity));
        })
        .synchronize()
        .schedule(|mut destroy: Destroy| destroy.destroy_all())
        .runner()
        .unwrap();
    runner.run(&mut world);
}
