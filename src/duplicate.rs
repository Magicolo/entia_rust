use std::{any::type_name, collections::HashSet};

use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::{Error, Result},
    inject::{Context, Get, Inject},
    world::{segment::Row, store::Store, World},
    write::{self, Write},
};

pub struct Duplicate<'a> {
    defer: defer::Defer<'a, Inner>,
    buffer: &'a mut Vec<Entity>,
    segments: &'a mut HashSet<usize>,
    entities: &'a mut Entities,
    world: &'a World,
}

pub struct State(defer::State<Inner>);

struct Inner {
    entities: write::State<Entities>,
    buffer: Vec<Entity>,
    segments: HashSet<usize>,
}

struct Defer {
    segment: usize,
    store: usize,
    entities: Vec<Entity>,
    row: Row,
}

impl Duplicate<'_> {
    pub fn one(&mut self, entity: Entity, count: usize) -> Result<&[Entity]> {
        if count == 0 {
            return Ok(&[]);
        }

        let datum = self
            .entities
            .get_datum(entity)
            .ok_or(Error::InvalidEntity(entity))?;
        let segment = &self.world.segments[datum.segment_index as usize];
        if segment.can_clone() {
            self.segments.insert(segment.index());
            self.buffer.resize(count, Entity::NULL);
            let ready = self.entities.reserve(self.buffer);
            let pair = segment.reserve(count);
            if ready < count || pair.1 < count {
                self.defer.defer(Defer {
                    segment: segment.index(),
                    entities: self.buffer.drain(..).collect(),
                    store: pair.0,
                    row: segment.row(datum.store_index as usize)?.extract()?,
                });
            } else {
                unsafe {
                    segment
                        .store_at(0)
                        .ok_or(Error::MissingStore(type_name::<Entity>(), segment.index()))?
                        .set_all(pair.0, self.buffer)
                };
                for store in segment.stores().skip(1) {
                    let source = (store, datum.store_index as usize);
                    let target = (store, pair.0);
                    unsafe { Store::clone(source, target, count) }?;
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
            Err(Error::All(
                segment
                    .stores()
                    .filter_map(|store| {
                        Some(Error::MissingClone(store.meta().name))
                            .filter(|_| store.meta().clone.is_none())
                    })
                    .collect(),
            )
            .flatten(false)
            .unwrap())
        }
    }
}

impl Inject for Duplicate<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, mut context: Context) -> Result<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        let inner = Inner {
            entities,
            segments: HashSet::new(),
            buffer: Vec::new(),
        };
        let defer = <defer::Defer<Inner> as Inject>::initialize(inner, context)?;
        Ok(State(defer))
    }

    fn resolve(State(state): &mut Self::State, mut context: Context) -> Result {
        <defer::Defer<Inner> as Inject>::resolve(state, context.owned())
    }
}

impl<'a> Get<'a> for State {
    type Item = Duplicate<'a>;

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

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) -> Result {
        let entities = self.entities.as_mut();
        entities.resolve();

        for segment in self.segments.drain() {
            world.segments[segment].resolve();
        }

        for defer in items {
            let count = defer.entities.len();
            let segment = &mut world.segments[defer.segment];

            unsafe {
                segment
                    .store_at(0)
                    .ok_or(Error::MissingStore(type_name::<Entity>(), segment.index()))?
                    .set_all(defer.store, &defer.entities)
            };

            for (i, store) in segment.stores().enumerate().skip(1) {
                let source = defer
                    .row
                    .store(i)
                    .ok_or(Error::MissingStore(store.meta().name, segment.index()))?;
                let source = (source, defer.row.index());
                let target = (store, defer.store);
                unsafe { Store::clone(source, target, count) }?;
            }

            for (i, &entity) in self.buffer.iter().enumerate() {
                entities
                    .get_datum_at_mut(entity.index())
                    .ok_or(Error::InvalidEntity(entity))?
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
        vec![Dependency::defer::<Entity>()]
    }
}
