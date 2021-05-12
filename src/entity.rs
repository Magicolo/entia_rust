use crate::inject::*;
use crate::item::*;
use crate::resource::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}
pub struct EntityState(Arc<Store<Entity>>, usize);

pub struct Entities {
    pub free: Vec<Entity>,
    pub last: AtomicU32,
    pub capacity: AtomicUsize,
    pub data: Vec<Datum>,
}
pub struct ReadState(Arc<Store<Entities>>, Arc<Segment>);
pub struct WriteState(Arc<Store<Entities>>, Arc<Segment>);

impl Entities {
    pub fn create<const N: usize>(&self) -> [Entity; N] {
        // TODO: use 'MaybeUninit'?
        let mut entities = [Entity::default(); N];
        entities
    }

    pub fn has(&self, entity: Entity) -> bool {
        match self.get_datum(entity) {
            Some(datum) => *unsafe { datum.store.at(datum.index as usize) } == entity,
            None => false,
        }
    }

    pub fn destroy(&mut self, entities: &[Entity]) -> usize {
        todo!()
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        if entity.index < self.last.load(Ordering::Relaxed) {
            Some(&self.data[entity.index as usize])
        } else {
            None
        }
    }
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            free: Vec::with_capacity(32),
            last: 0.into(),
            capacity: 0.into(),
            data: Vec::with_capacity(32),
        }
    }
}

impl Inject for &Entities {
    type State = ReadState;

    fn initialize(world: &mut World) -> Option<Self::State> {
        initialize(Entities::default, world).map(|pair| ReadState(pair.0, pair.1))
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = vec![Dependency::Read(state.1.index, TypeId::of::<Entities>())];
        for segment in world.segments.iter() {
            dependencies.push(Dependency::Read(segment.index, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a> Get<'a> for ReadState {
    type Item = &'a Entities;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}

impl Inject for &mut Entities {
    type State = WriteState;

    fn initialize(world: &mut World) -> Option<Self::State> {
        initialize(Entities::default, world).map(|pair| WriteState(pair.0, pair.1))
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = vec![Dependency::Write(state.1.index, TypeId::of::<Entities>())];
        for segment in world.segments.iter() {
            dependencies.push(Dependency::Write(segment.index, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a> Get<'a> for WriteState {
    type Item = &'a mut Entities;

    fn get(&'a mut self, _: &World) -> Self::Item {
        unsafe { self.0.at(0) }
    }
}

impl Item for Entity {
    type State = EntityState;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(EntityState(segment.store()?, segment.index))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Read(state.1, TypeId::of::<Entity>())]
    }
}

impl At<'_> for EntityState {
    type Item = Entity;

    #[inline]
    fn at(&self, index: usize) -> Self::Item {
        unsafe { *self.0.at(index) }
    }
}
