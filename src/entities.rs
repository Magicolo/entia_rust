use crate::entity::*;
use crate::inject::*;
use crate::resource::*;
use crate::system::*;
use crate::world::*;
use crate::write;
use crate::write::*;
use std::any::TypeId;
use std::sync::atomic::AtomicU32;

pub struct Entities<'a>(&'a mut Inner);
pub struct State(write::State<Inner>);
struct Inner {
    pub free: Vec<Entity>,
    pub last: AtomicU32,
    pub data: Vec<Datum>,
}

impl Resource for Inner {}

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
        self.0.data.get(entity.index as usize)
    }

    pub fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.0.data.get_mut(entity.index as usize)
    }
}

impl State {
    pub fn entities(&mut self) -> Entities {
        Entities(self.0.as_mut())
    }
}

impl Inner {
    fn new(capacity: usize) -> Self {
        Inner {
            free: Vec::with_capacity(capacity),
            last: 0.into(),
            data: Vec::with_capacity(capacity),
        }
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