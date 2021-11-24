use super::*;

#[test]
fn has_entity_count() -> Result {
    let mut world = world();
    let mut create = world.injector::<Create<_>>()?;
    let mut query = world.injector::<Query<(Entity, &Position)>>()?;
    let mut families = world.injector::<Families>()?;
    let entity = {
        let mut guard = create.guard(&mut world)?;
        let mut create = guard.inject();
        create.one(Add::new(Position(1., 2., 3.))).entity()
    };
    {
        let mut guard = query.guard(&mut world)?;
        let query = guard.inject();
        assert!(matches!(query.get(entity), Some(_)));
        assert_eq!(query.len(), 1);
        for item in &query {
            assert_eq!(entity, item.0);
        }
    }
    {
        let mut guard = families.guard(&mut world)?;
        let families = guard.inject();
        assert_eq!(families.roots().count(), 1);
        for root in families.roots() {
            assert_eq!(entity, root.entity());
        }
    }
    Ok(())
}
