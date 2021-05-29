use std::sync::Mutex;

use crate::{
    entity::Entity,
    inject::{Get, Inject},
    resource::Resource,
    system::Dependency,
    world::World,
    write::{self, Write},
};

pub struct Entities<'a>(&'a mut Inner);
pub struct State(write::State<Inner>);

#[derive(Default)]
pub struct Datum {
    pub(crate) index: u32,
    pub(crate) segment: u32,
    pub(crate) generation: u32,
    pub(crate) state: u8,
}

struct Inner {
    pub free: Vec<Entity>,
    pub data: Vec<Datum>,
    pub lock: Mutex<()>,
}

impl Resource for Inner {}

impl Entities<'_> {
    pub fn reserve(&mut self, entities: &mut [Entity]) {
        let _ = self.0.lock.lock().unwrap();
        self.0.reserve(entities);
    }

    #[inline]
    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.0.get_datum(entity)
    }
}

impl State {
    #[inline]
    pub fn release(&mut self, entities: &[Entity]) {
        self.0.as_mut().release(entities);
    }

    #[inline]
    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.0.as_ref().get_datum(entity)
    }

    #[inline]
    pub fn get_datum_at_mut(&mut self, index: usize) -> Option<&mut Datum> {
        self.0.as_mut().data.get_mut(index)
    }

    #[inline]
    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.0.as_mut().get_datum_mut(entity)
    }
}

impl Inner {
    pub fn new(capacity: usize) -> Self {
        Inner {
            free: Vec::with_capacity(capacity),
            data: Vec::with_capacity(capacity),
            lock: Mutex::new(()),
        }
    }

    pub fn reserve(&mut self, entities: &mut [Entity]) {
        let mut current = 0;
        while current < entities.len() {
            if let Some(mut entity) = self.free.pop() {
                let datum = &mut self.data[entity.index as usize];
                datum.generation += 1;
                datum.state = 1;
                entity.generation = datum.generation;
                entities[current] = entity;
                current += 1;
            } else {
                break;
            }
        }

        while current < entities.len() {
            let index = self.data.len();
            let datum = Datum {
                index: 0,
                segment: 0,
                generation: 0,
                state: 1,
            };
            entities[current] = Entity {
                index: index as u32,
                generation: datum.generation,
            };
            self.data.push(datum);
            current += 1;
        }
    }

    #[inline]
    pub fn release(&mut self, entities: &[Entity]) {
        for entity in entities {
            self.data[entity.index as usize].state = 0;
        }
        self.free.extend_from_slice(entities);
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data
            .get(entity.index as usize)
            .filter(|datum| entity.generation == datum.generation)
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data
            .get_mut(entity.index as usize)
            .filter(|datum| entity.generation == datum.generation)
    }
}

impl Default for Inner {
    fn default() -> Self {
        Self::new(32)
    }
}

impl Inject for Entities<'_> {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Write<Inner> as Inject>::initialize(None, world).map(|state| State(state))
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        <Write<Inner> as Inject>::depend(&state.0, world)
    }
}

impl<'a> From<&'a mut State> for Entities<'a> {
    #[inline]
    fn from(state: &'a mut State) -> Self {
        Entities(state.0.as_mut())
    }
}

impl<'a> Get<'a> for State {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(self.0.get(world))
    }
}
