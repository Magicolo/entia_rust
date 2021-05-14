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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}
pub struct EntityState(Arc<Store<Entity>>, usize);

pub struct Entities<'a>(&'a mut EntitiesInner, &'a World);
pub struct EntitiesState(Arc<Store<EntitiesInner>>, Arc<Segment>);
struct EntitiesInner {
    pub free: Vec<Entity>,
    pub last: AtomicU32,
    pub capacity: AtomicUsize,
    pub data: Vec<Datum>,
}
// impl Resource for EntitiesInner {}

impl Entity {
    pub const ZERO: Self = Self {
        index: 0,
        generation: 0,
    };
}

impl Default for Entity {
    #[inline]
    fn default() -> Self {
        Self::ZERO
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

impl Entities<'_> {
    pub fn create<const N: usize>(&self) -> [Entity; N] {
        // TODO: use 'MaybeUninit'?
        // let mut entities = [Entity::ZERO; N];
        // entities
        todo!()
    }

    pub fn has(&self, entity: Entity) -> bool {
        match self.get_datum(entity) {
            Some(datum) => *unsafe { datum.store.at(datum.index as usize) } == entity,
            None => false,
        }
    }

    pub fn destroy(&mut self, _entities: &[Entity]) -> usize {
        todo!()
    }

    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        if entity.index < self.0.last.load(Ordering::Relaxed) {
            Some(&self.0.data[entity.index as usize])
        } else {
            None
        }
    }
}

impl Default for EntitiesInner {
    fn default() -> Self {
        Self {
            free: Vec::with_capacity(32),
            last: 0.into(),
            capacity: 0.into(),
            data: Vec::with_capacity(32),
        }
    }
}

impl Inject for Entities<'_> {
    type Input = ();
    type State = EntitiesState;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        initialize(EntitiesInner::default, world).map(|pair| EntitiesState(pair.0, pair.1))
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = vec![Dependency::Write(state.1.index, TypeId::of::<Entities>())];
        for segment in world.segments.iter() {
            dependencies.push(Dependency::Write(segment.index, TypeId::of::<Entity>()));
        }
        dependencies
    }
}

impl<'a> Get<'a> for EntitiesState {
    type Item = Entities<'a>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Entities(unsafe { self.0.at(0) }, world)
    }
}
