use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Get, Inject, InjectContext},
    world::World,
    write::{self, Write},
};
use std::{any::TypeId, collections::HashSet};

pub struct Destroy<'a> {
    defer: &'a mut Vec<Entity>,
}

pub struct State {
    defer: Vec<Entity>,
    set: HashSet<Entity>,
    entities: write::State<Entities>,
}

impl Destroy<'_> {
    #[inline]
    pub fn one(&mut self, entity: Entity) {
        self.defer.push(entity);
    }

    #[inline]
    pub fn all(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.defer.extend(entities);
    }
}

unsafe impl Inject for Destroy<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, context: InjectContext) -> Option<Self::State> {
        Some(State {
            defer: Vec::new(),
            set: HashSet::new(),
            entities: <Write<Entities> as Inject>::initialize(None, context)?,
        })
    }

    fn resolve(state: &mut Self::State, mut context: InjectContext) {
        let entities = state.entities.as_mut();
        let world = context.world();
        let set = &mut state.set;
        set.clear();

        for entity in state.defer.drain(..) {
            if entities.has(entity) {
                destroy(entity.index, true, set, entities, world);
            }
        }

        if set.len() > 0 {
            entities.release(set.drain());
        }

        fn destroy(
            index: u32,
            root: bool,
            set: &mut HashSet<Entity>,
            entities: &mut Entities,
            world: &mut World,
        ) -> Option<u32> {
            if let Some(datum) = entities.data.0.get(index as usize).cloned() {
                let entity = Entity {
                    index,
                    generation: datum.generation,
                };

                if set.insert(entity) {
                    let mut child = datum.first_child;
                    while let Some(next) = destroy(child, false, set, entities, world) {
                        child = next;
                    }

                    if root {
                        if let Some(previous_sibling) =
                            entities.data.0.get_mut(datum.previous_sibling as usize)
                        {
                            previous_sibling.next_sibling = datum.next_sibling;
                        } else if let Some(parent) = entities.data.0.get_mut(datum.parent as usize)
                        {
                            // Only an entity with no 'previous_sibling' can ever be the 'first_child' of its parent.
                            parent.first_child = datum.next_sibling;
                        }

                        if let Some(next_sibling) =
                            entities.data.0.get_mut(datum.next_sibling as usize)
                        {
                            next_sibling.previous_sibling = datum.previous_sibling;
                        }
                    }

                    let segment = &mut world.segments[datum.segment_index as usize];
                    if segment.remove_at(datum.store_index as usize) {
                        // SAFETY: When it exists, the entity store is always the first. This segment must have
                        // an entity store since the destroyed entity was in it.
                        let moved =
                            *unsafe { segment.stores[0].get::<Entity>(datum.store_index as usize) };
                        entities.data.0[moved.index as usize]
                            .update(datum.store_index, datum.segment_index);
                    }
                }

                Some(datum.next_sibling)
            } else {
                None
            }
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
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for segment in world.segments.iter() {
            dependencies.push(Dependency::Defer(segment.index, TypeId::of::<Entity>()));
        }
        dependencies
    }
}
