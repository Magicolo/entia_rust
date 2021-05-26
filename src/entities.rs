use std::{
    any::TypeId,
    sync::{atomic::AtomicU32, Arc},
};

use crate::{
    entity::Entity,
    inject::{Get, Inject},
    resource::Resource,
    system::Dependency,
    world::Store,
    world::World,
    write::{self, Write},
};

pub struct Entities<'a>(&'a mut Inner);
pub struct State(write::State<Inner>);

pub struct Datum {
    pub(crate) index: u32,
    pub(crate) segment: u32,
    pub(crate) store: Arc<Store<Entity>>,
}

struct Inner {
    pub free: Vec<Entity>,
    pub last: AtomicU32,
    pub data: Vec<Datum>,
}

impl Resource for Inner {}

impl Entities<'_> {
    pub fn reserve<const N: usize>(&self) -> [Entity; N] {
        // TODO: use 'MaybeUninit'?
        // let mut entities = [Entity::ZERO; N];
        // entities
        todo!()
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.0
            .data
            .get(entity.index as usize)
            .filter(|datum| *unsafe { datum.store.at(datum.index as usize) } == entity)
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
    pub unsafe fn get_datum_at_mut(&mut self, index: usize) -> &mut Datum {
        &mut self.0.as_mut().data[index]
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
            last: 0.into(),
            data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn release(&mut self, entities: &[Entity]) {
        self.free.extend_from_slice(entities);
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data
            .get(entity.index as usize)
            .filter(|datum| *unsafe { datum.store.at(datum.index as usize) } == entity)
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data
            .get_mut(entity.index as usize)
            .filter(|datum| *unsafe { datum.store.at(datum.index as usize) } == entity)
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
        let mut dependencies = <Write<Inner> as Inject>::depend(&state.0, world);
        for segment in world.segments.iter() {
            dependencies.push(Dependency::Write(segment.index, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a> Get<'a> for State {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(self.0.get(world))
    }
}
