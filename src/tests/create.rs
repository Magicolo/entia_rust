use entia_check::{FullGenerator, Generator, IntoGenerator};
use std::collections::HashSet;

use super::*;

#[test]
fn has_entity_count() -> Result {
    let position = <(f64, f64, f64)>::generator().map(|(x, y, z)| Position(x, y, z));
    let count = (0usize..1000).generator();
    for (position, count) in (position, count).sample(100) {
        let mut world = world();
        let mut create = world.injector::<Create<_>>()?;
        let mut query = world.injector::<Query<(Entity, &Position)>>()?;
        let mut families = world.injector::<Families>()?;

        let entities: HashSet<_> = create.run(&mut world, |mut create| {
            create
                .clones(count, Add::new(position.clone()))
                .roots()
                .map(|family| family.entity())
                .collect()
        })?;

        query.run(&mut world, |query| {
            for &entity in entities.iter() {
                let item = query.get(entity).unwrap();
                assert_eq!(entity, item.0);
                assert_eq!(&position, item.1);
            }
            assert_eq!(query.len(), count);
            for item in &query {
                assert!(entities.contains(&item.0));
                assert_eq!(&position, item.1);
            }
        })?;

        families.run(&mut world, |families| {
            assert_eq!(families.roots().count(), count);
            for root in families.roots() {
                assert!(entities.contains(&root.entity()));
            }
        })?;
    }
    Ok(())
}
