use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Context, Get, Inject},
    resource::Resource,
    world::World,
    write::{self, Write},
};

pub struct Entities<'a>(&'a mut Inner);
#[derive(Clone)]
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
    pub fn reserve(&mut self) -> Option<u32> {
        if self.state == Self::RELEASED {
            self.generation += 1;
            self.state = Self::RESERVED;
            Some(self.generation)
        } else {
            None
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
    #[inline]
    pub fn reserve(&mut self, entities: &mut [Entity]) {
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
    #[allow(dead_code)]
    pub fn get_datum(&mut self, entity: Entity) -> Option<&Datum> {
        self.0.as_ref().get_datum(entity)
    }

    #[inline]
    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.0.as_mut().get_datum_mut(entity)
    }

    #[inline]
    #[allow(dead_code)]
    pub unsafe fn get_datum_unchecked(&mut self, entity: Entity) -> &Datum {
        self.0.as_ref().get_datum_unchecked(entity)
    }

    #[inline]
    pub unsafe fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        self.0.as_mut().get_datum_mut_unchecked(entity)
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
        // TODO: make this thread safe
        let mut current = 0;
        while current < entities.len() {
            if let Some(mut entity) = self.free.pop() {
                let datum = &mut self.data[entity.index as usize];
                if let Some(generation) = datum.reserve() {
                    entity.generation = generation;
                    entities[current] = entity;
                    current += 1;
                }
            } else {
                break;
            }
        }

        while current < entities.len() {
            let index = self.data.len();
            self.data.push(Datum {
                index: 0,
                segment: 0,
                generation: 0,
                state: 1,
            });
            entities[current] = Entity {
                index: index as u32,
                generation: 0,
            };
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
        self.data.get(entity.index as usize).filter(|datum| {
            entity.generation == datum.generation && datum.state == Datum::INITIALIZED
        })
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data.get_mut(entity.index as usize).filter(|datum| {
            entity.generation == datum.generation && datum.state == Datum::INITIALIZED
        })
    }

    #[inline]
    pub unsafe fn get_datum_unchecked(&self, entity: Entity) -> &Datum {
        &self.data[entity.index as usize]
    }

    #[inline]
    pub unsafe fn get_datum_mut_unchecked(&mut self, entity: Entity) -> &mut Datum {
        &mut self.data[entity.index as usize]
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

    fn initialize(_: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let inner = <Write<Inner> as Inject>::initialize(None, context, world)?;
        Some(State(inner))
    }
}

impl<'a> Get<'a> for State {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(self.0.get(world))
    }
}

unsafe impl Depend for State {
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
