use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error,
    inject::{Context, Get, Inject},
    world::{segment::Segment, store::Store, World},
    write::Write,
};
use std::collections::HashSet;

pub struct Duplicate<'a> {
    defer: defer::Defer<'a, Inner>,
    buffer: &'a mut Vec<Entity>,
    segments: &'a mut HashSet<usize>,
    entities: &'a mut Entities,
    world: &'a World,
}

pub struct State(defer::State<Inner>);

#[derive(Debug)]
pub enum Error {
    WrongSegment,
    InvalidEntity(Entity),
    MissingClone(usize),
    SegmentIndexOutOfRange(usize),
    SegmentMustBeClonable(usize, usize),
}

struct Slot(Vec<Store>);
struct Inner {
    entities: Write<Entities>,
    buffer: Vec<Entity>,
    segments: HashSet<usize>,
}

struct Defer {
    segment: usize,
    store: usize,
    entities: Vec<Entity>,
    slot: Slot,
}

error::error!(Error, error::Error::Duplicate);

impl Duplicate<'_> {
    pub fn one(&mut self, entity: impl Into<Entity>, count: usize) -> Result<&[Entity], Error> {
        if count == 0 {
            return Ok(&[]);
        }

        let entity = entity.into();
        let datum = self
            .entities
            .get_datum(entity)
            .ok_or(Error::InvalidEntity(entity))?;
        let segment = &self.world.segments()[datum.segment_index as usize];
        if segment.can_clone() {
            self.segments.insert(segment.index());
            self.buffer.resize(count, Entity::NULL);
            let ready = self.entities.reserve(self.buffer);
            let pair = segment.reserve(count);
            if ready < count || pair.1 < count {
                self.defer.one(Defer {
                    segment: segment.index(),
                    entities: self.buffer.drain(..).collect(),
                    store: pair.0,
                    slot: Slot::get(segment, datum.store_index as usize)
                        .expect("Segment must be able to get slot."),
                });
            } else {
                unsafe { segment.entity_store().set_all(pair.0, self.buffer) };
                for store in segment.component_stores() {
                    let source = (store, datum.store_index as usize);
                    unsafe { Store::fill(source, (store, pair.0), 1) }
                        .expect("Store must be clonable.");
                }

                // TODO: Calling 'initialize' may make the entities visible. There should be a way to distinguish pending entities
                // from ready entities. A flag field?
                for (i, &entity) in self.buffer.iter().enumerate() {
                    self.entities
                        .get_datum_at_mut(entity.index())
                        .unwrap()
                        .initialize(
                            entity.generation(),
                            (pair.0 + i) as u32,
                            segment.index() as u32,
                            None,
                            None,
                            None,
                            None,
                            None,
                        );
                }
            }
            Ok(&self.buffer)
        } else {
            Err(Error::MissingClone(segment.index()))
        }
    }
}

impl Slot {
    fn get(segment: &Segment, index: usize) -> Result<Slot, error::Error> {
        if index >= segment.count() {
            Err(error::Error::SegmentIndexOutOfRange {
                index,
                segment: segment.index(),
            })
        } else if segment.can_clone() {
            let sources = segment.component_stores();
            let mut targets = Vec::with_capacity(sources.len());
            for store in sources {
                targets.push(unsafe { store.chunk(index, 1) }?);
            }
            Ok(Slot(targets))
        } else {
            Err(error::Error::SegmentMustBeClonable {
                segment: segment.index(),
            })
        }
    }

    fn set(self, segment: &Segment, index: usize, count: usize) {
        if count == 0 {
            for store in self.0 {
                unsafe { store.free(1, 1) };
            }
        } else {
            for (target, source) in segment.component_stores().zip(self.0) {
                // First index copies to unify behavior between the deferred and non-deferred version.
                // If 'Clone' was used, there would be 1 additionnal 'Clone' and 'Drop' in the deferred version.
                unsafe { Store::copy((&source, 0), (target, index), 1) };
                unsafe { Store::fill((&source, 0), (target, index + 1), count - 1) }
                    .expect("Store must be clonable.");
                // Since the memory at index 0 has been copied over, it must not be dropped.
                unsafe { source.free(0, 1) };
            }
        }
    }
}

impl Inject for Duplicate<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State, error::Error> {
        let entities = Write::initialize(None, context.owned())?;
        let inner = Inner {
            entities,
            segments: HashSet::new(),
            buffer: Vec::new(),
        };
        Ok(State(defer::Defer::initialize(inner, context)?))
    }

    fn resolve(State(state): &mut Self::State, mut context: Context) -> Result<(), error::Error> {
        defer::Defer::resolve(state, context.owned())
    }
}

impl<'a> Get<'a> for State {
    type Item = Duplicate<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        let (defer, inner) = self.0.get(world);
        Duplicate {
            defer,
            buffer: &mut inner.buffer,
            segments: &mut inner.segments,
            entities: inner.entities.get(world),
            world,
        }
    }
}

impl Resolve for Inner {
    type Item = Defer;

    fn resolve(
        &mut self,
        items: impl Iterator<Item = Self::Item>,
        world: &mut World,
    ) -> Result<(), error::Error> {
        let entities = self.entities.as_mut();
        let segments = world.segments_mut();
        entities.resolve();

        for segment in self.segments.drain() {
            segments[segment].resolve();
        }

        for defer in items {
            let count = defer.entities.len();
            let segment = &mut segments[defer.segment];
            unsafe { segment.entity_store().set_all(defer.store, &defer.entities) };
            defer.slot.set(segment, defer.store, count);

            for (i, &entity) in self.buffer.iter().enumerate() {
                entities
                    .get_datum_at_mut(entity.index())
                    .expect("Entity index must be in bounds.")
                    .initialize(
                        entity.generation(),
                        (defer.store + i) as u32,
                        segment.index() as u32,
                        None,
                        None,
                        None,
                        None,
                        None,
                    );
            }
        }

        Ok(())
    }
}

unsafe impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        // TODO: These dependencies do not take into account that the 'Duplicate' module may read from any component...
        // - See 'Slot::get'
        vec![Dependency::defer::<Entity>()]
    }
}
