use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::{Error, Result},
    inject::{Context, Get, Inject},
    world::World,
    write::Write,
};
use std::{any::type_name, collections::HashSet};

pub struct Destroy<'a>(defer::Defer<'a, Inner>);
pub struct State(defer::State<Inner>);

struct Inner {
    set: HashSet<Entity>,
    entities: Write<Entities>,
}

enum Defer {
    One(Entity),
    Family(Entity),
}

impl Destroy<'_> {
    #[inline]
    pub fn one(&mut self, entity: Entity) {
        self.0.defer(Defer::One(entity));
    }

    #[inline]
    pub fn all(&mut self, entities: impl Iterator<Item = Entity>) {
        for entity in entities {
            self.one(entity);
        }
    }

    #[inline]
    pub fn family(&mut self, entity: Entity) {
        self.0.defer(Defer::Family(entity));
    }
}

impl Inject for Destroy<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let inner = Inner {
            set: HashSet::new(),
            entities: <Write<Entities> as Inject>::initialize(None, context.owned())?,
        };
        let defer = <defer::Defer<Inner> as Inject>::initialize(inner, context)?;
        Ok(State(defer))
    }

    fn resolve(State(state): &mut Self::State, context: Context) -> Result {
        <defer::Defer<Inner> as Inject>::resolve(state, context)
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) -> Result {
        let entities = self.entities.as_mut();
        let set = &mut self.set;

        for defer in items {
            match defer {
                Defer::One(entity) if entities.has(entity) => {
                    destroy(entity.index(), true, false, set, entities, world)?;
                }
                Defer::Family(root) if entities.has(root) => {
                    destroy(root.index(), true, true, set, entities, world)?;
                }
                _ => {}
            }
        }

        if set.len() > 0 {
            entities.release(set.drain());
        }

        fn destroy(
            index: u32,
            root: bool,
            family: bool,
            set: &mut HashSet<Entity>,
            entities: &mut Entities,
            world: &mut World,
        ) -> Result<Option<u32>> {
            if let Some(datum) = entities.get_datum_at(index).cloned() {
                if set.insert(datum.entity(index)) {
                    if family {
                        let mut child = datum.first_child;
                        while let Some(next) = destroy(child, false, family, set, entities, world)?
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
                            // Only an entity with no 'previous_sibling' can ever be the 'first_child' of its parent.
                            parent.first_child = datum.next_sibling;
                        }

                        if let Some(next_sibling) = entities.get_datum_at_mut(datum.next_sibling) {
                            next_sibling.previous_sibling = datum.previous_sibling;
                        }
                    }

                    let segment = &mut world.segments[datum.segment_index as usize];
                    if segment.remove_at(datum.store_index as usize) {
                        // SAFETY: When it exists, the entity store is always the first. This segment must have
                        // an entity store since the destroyed entity was in it.
                        let entity = *unsafe {
                            segment
                                .store_at(0)
                                .ok_or(Error::MissingStore(type_name::<Entity>(), segment.index()))?
                                .get::<Entity>(datum.store_index as usize)
                        };
                        entities
                            .get_datum_at_mut(entity.index())
                            .ok_or(Error::InvalidEntity(entity))?
                            .update(&datum);
                    }
                }

                Ok(Some(datum.next_sibling))
            } else {
                Ok(None)
            }
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
