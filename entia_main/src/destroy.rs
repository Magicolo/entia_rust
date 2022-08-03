use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::{Error, Result},
    inject::{Context, Get, Inject},
    resource::Write,
    world::World,
};
use entia_core::FullIterator;
use std::{collections::HashSet, marker::PhantomData};

/// Resolves destroy operations such that coherence rules are strictly maintained
/// at the cost of likely causing a synchronization point.
/// Can be used as the resolution parameter of the 'Destroy' type.
pub struct Early;

/// Resolves destroy operations at the next synchronization point without adding
/// additionnal dependencies at the cost of allowing further systems
/// to observe a destroyed entity (with its state intact).
/// Can be used as the resolution parameter of the 'Destroy' type.
pub struct Late;

pub struct Destroy<'a, R = Early>(defer::Defer<'a, Inner>, PhantomData<R>);
pub struct State<R>(defer::State<Inner>, PhantomData<R>);

struct Inner {
    set: HashSet<Entity>,
    entities: Write<Entities>,
}

struct Defer {
    entity: Entity,
    descendants: bool,
}

impl<R> Destroy<'_, R> {
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

impl<R> Inject for Destroy<'_, R>
where
    State<R>: Depend,
{
    type Input = ();
    type State = State<R>;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let inner = Inner {
            set: HashSet::new(),
            entities: Write::initialize(None, context.owned())?,
        };
        Ok(State(
            defer::Defer::initialize(inner, context)?,
            PhantomData,
        ))
    }

    fn resolve(State(state, _): &mut Self::State, context: Context) -> Result {
        defer::Defer::resolve(state, context)
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(
        &mut self,
        items: impl FullIterator<Item = Self::Item>,
        world: &mut World,
    ) -> Result {
        fn destroy(
            index: u32,
            root: bool,
            descendants: bool,
            set: &mut HashSet<Entity>,
            entities: &mut Entities,
            world: &mut World,
        ) -> Result<Option<u32>> {
            // Entity index must be validated by caller.
            let datum = match entities.get_datum_at(index) {
                Some(datum) => datum.clone(),
                None => return Ok(None),
            };
            if set.insert(datum.entity(index)) {
                if descendants {
                    let mut child = datum.first_child;
                    while let Some(next) = destroy(child, false, descendants, set, entities, world)?
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
                    if !entities
                        .get_datum_at_mut(entity.index())
                        .expect("Entity must be valid.")
                        .update(datum.store_index, datum.segment_index)
                    {
                        return Err(Error::FailedToUpdate {
                            entity: entity.index(),
                            store: datum.store_index,
                            segment: datum.segment_index,
                        });
                    }
                }
            }

            Ok(Some(datum.next_sibling))
        }

        for Defer {
            entity,
            descendants,
        } in items
        {
            if self.entities.has(entity) {
                destroy(
                    entity.index(),
                    true,
                    descendants,
                    &mut self.set,
                    &mut self.entities,
                    world,
                )?;
            }
        }

        if self.set.len() > 0 {
            self.entities.release(self.set.drain());
        }

        Ok(())
    }
}

impl<'a, R> Get<'a> for State<R> {
    type Item = Destroy<'a, R>;

    #[inline]
    unsafe fn get(&'a mut self, world: &'a World) -> Self::Item {
        Destroy(self.0.get(world).0, PhantomData)
    }
}

unsafe impl Depend for State<Early> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        dependencies.push(Dependency::defer::<Entities>());
        dependencies.push(Dependency::defer::<Entity>());
        dependencies
    }
}

unsafe impl Depend for State<Late> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.0.depend(world)
    }
}
