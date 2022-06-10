use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    inject::{Context, Get, Inject},
    world::World,
    write::Write,
};
use std::collections::HashSet;

pub struct Destroy<'a>(defer::Defer<'a, Inner>);
pub struct State(defer::State<Inner>);

struct Inner {
    set: HashSet<Entity>,
    entities: Write<Entities>,
}

struct Defer {
    entity: Entity,
    descendants: bool,
}

impl Destroy<'_> {
    #[inline]
    pub fn one(&mut self, entity: impl Into<Entity>, descendants: bool) {
        self.0.one(Defer {
            entity: entity.into(),
            descendants,
        });
    }

    #[inline]
    pub fn all(
        &mut self,
        entities: impl IntoIterator<Item = impl Into<Entity>>,
        descendants: bool,
    ) {
        self.0.all(entities.into_iter().map(|entity| Defer {
            entity: entity.into(),
            descendants,
        }))
    }
}

impl Inject for Destroy<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let inner = Inner {
            set: HashSet::new(),
            entities: Write::initialize(None, context.owned())?,
        };
        Ok(State(defer::Defer::initialize(inner, context)?))
    }

    fn resolve(State(state): &mut Self::State, context: Context) -> Result {
        defer::Defer::resolve(state, context)
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) -> Result {
        fn destroy(
            index: u32,
            root: bool,
            descendants: bool,
            set: &mut HashSet<Entity>,
            entities: &mut Entities,
            world: &mut World,
        ) -> Option<u32> {
            // Entity index must be validated by caller.
            let datum = entities.get_datum_at(index).cloned()?;
            if set.insert(datum.entity(index)) {
                if descendants {
                    let mut child = datum.first_child;
                    while let Some(next) = destroy(child, false, descendants, set, entities, world)
                    {
                        child = next;
                    }
                }

                if root {
                    if let Some(previous_sibling) =
                        entities.get_datum_at_mut(datum.previous_sibling)
                    {
                        previous_sibling.next_sibling = datum.next_sibling;
                    } else if let Some(parent) = entities.get_datum_at_mut(datum.parent) {
                        // Only an entity with no 'previous_sibling' can not be the 'first_child' of its parent.
                        parent.first_child = datum.next_sibling;
                    }

                    if let Some(next_sibling) = entities.get_datum_at_mut(datum.next_sibling) {
                        next_sibling.previous_sibling = datum.previous_sibling;
                    }
                }

                let segment = &mut world.segments_mut()[datum.segment_index as usize];
                // TODO: There may be a way to batch these removals.
                if segment.remove_at(datum.store_index as usize) {
                    let entity = *unsafe {
                        segment
                            .entity_store()
                            .get::<Entity>(datum.store_index as usize)
                    };
                    entities
                        .get_datum_at_mut(entity.index())
                        .expect("Entity must be valid.")
                        .update(&datum);
                }
            }

            Some(datum.next_sibling)
        }

        let entities = self.entities.as_mut();
        for Defer {
            entity,
            descendants,
        } in items
        {
            if entities.has(entity) {
                destroy(
                    entity.index(),
                    true,
                    descendants,
                    &mut self.set,
                    entities,
                    world,
                );
            }
        }

        if self.set.len() > 0 {
            entities.release(self.set.drain());
        }

        Ok(())
    }
}

impl<'a> Get<'a> for State {
    type Item = Destroy<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy(self.0.get(world).0)
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        dependencies.push(Dependency::defer::<Entities>());
        dependencies.push(Dependency::defer::<Entity>());
        dependencies
    }
}
