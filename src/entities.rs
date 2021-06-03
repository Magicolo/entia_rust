use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Get, Inject},
    resource::Resource,
    world::World,
    write::{self, Write},
};

pub struct Entities<'a>(&'a mut Inner);
pub struct State(write::State<Inner>);

#[derive(Default)]
pub struct Datum {
    index: u32,
    segment: u32,
    generation: u32,
    state: u8,
}

struct Inner {
    pub free: Vec<Entity>,
    pub data: Vec<Datum>,
}

impl Resource for Inner {}

impl Datum {
    const RELEASED: u8 = 0;
    const RESERVED: u8 = 1;
    const INITIALIZED: u8 = 2;

    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn segment(&self) -> u32 {
        self.segment
    }

    #[inline]
    pub fn reserve(&mut self, generation: u32) -> bool {
        if self.state == Self::RELEASED {
            self.generation = generation;
            self.state = Self::RESERVED;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn release(&mut self) -> bool {
        if self.state == Self::INITIALIZED {
            self.state = Self::RELEASED;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn initialize(&mut self, index: u32, segment: u32) -> bool {
        if self.state == Self::RESERVED {
            self.index = index;
            self.segment = segment;
            self.state = Self::INITIALIZED;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn update(&mut self, index: u32, segment: u32) -> bool {
        if self.state == Self::INITIALIZED {
            self.index = index;
            self.segment = segment;
            true
        } else {
            false
        }
    }
}

impl Entities<'_> {
    pub fn reserve(&mut self, entities: &mut [Entity]) {
        // TODO: Make this thread-safe
        // let guard = self.0.lock.lock().unwrap();
        self.0.reserve(entities);
        // drop(guard);
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
    pub fn get_datum_at_mut(&mut self, index: usize) -> &mut Datum {
        &mut self.0.as_mut().data[index]
    }

    #[inline]
    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.0.as_mut().get_datum_mut(entity)
    }
}

impl Inner {
    pub fn new(capacity: usize) -> Self {
        let mut free = Vec::with_capacity(capacity);
        let mut data = Vec::with_capacity(capacity);
        free.push(Entity::ZERO);
        data.push(Datum::default());
        Inner { free, data }
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
            self.data[entity.index as usize].release();
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
    #[inline]
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
}

impl<'a> Get<'a> for State {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(self.0.get(world))
    }
}

impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        self.0.depend(world)
    }
}

impl<'a> From<&'a mut State> for Entities<'a> {
    #[inline]
    fn from(state: &'a mut State) -> Self {
        Entities(state.0.as_mut())
    }
}
