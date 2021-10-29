use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Context, Get, Inject},
    world::World,
    write::{self, Write},
};
use std::collections::HashSet;

pub struct Destroy<'a> {
    defer: &'a mut Vec<Defer>,
}

pub struct State {
    defer: Vec<Defer>,
    set: HashSet<Entity>,
    entities: write::State<Entities>,
}

enum Defer {
    One(Entity),
    Family(Entity),
}

impl Destroy<'_> {
    #[inline]
    pub fn one(&mut self, entity: Entity) {
        self.defer.push(Defer::One(entity));
    }

    #[inline]
    pub fn all(&mut self, entities: impl Iterator<Item = Entity>) {
        for entity in entities {
            self.one(entity);
        }
    }

    #[inline]
    pub fn family(&mut self, entity: Entity) {
        self.defer.push(Defer::Family(entity));
    }
}

impl Inject for Destroy<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, context: Context) -> Option<Self::State> {
        Some(State {
            defer: Vec::new(),
            set: HashSet::new(),
            entities: <Write<Entities> as Inject>::initialize(None, context)?,
        })
    }

    fn resolve(state: &mut Self::State, mut context: Context) {
        let entities = state.entities.as_mut();
        let world = context.world();
        let set = &mut state.set;

        for defer in state.defer.drain(..) {
            match defer {
                Defer::One(entity) if entities.has(entity) => {
                    destroy(entity.index(), true, false, set, entities, world);
                }
                Defer::Family(root) if entities.has(root) => {
                    destroy(root.index(), true, true, set, entities, world);
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
        ) -> Option<u32> {
            let datum = entities.get_datum_at(index)?.clone();
            let entity = datum.entity(index);
            if set.insert(entity) {
                if family {
                    let mut child = datum.first_child;
                    while let Some(next) = destroy(child, false, family, set, entities, world) {
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
                    let moved =
                        *unsafe { segment.stores[0].get::<Entity>(datum.store_index as usize) };
                    entities
                        .get_datum_at_mut(moved.index())
                        .unwrap()
                        .update(datum.store_index, datum.segment_index);
                }
            }

            Some(datum.next_sibling)
        }
    }
}

impl<'a> Get<'a> for State {
    type Item = Destroy<'a>;

    #[inline]
    fn get(&'a mut self, _: &'a World) -> Self::Item {
        Destroy {
            defer: &mut self.defer,
        }
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![
            Dependency::defer::<Entities>(),
            Dependency::defer::<Entity>(),
        ]
    }
}
