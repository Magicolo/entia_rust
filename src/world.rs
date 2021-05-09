use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

/*
SYSTEMS
- Runners must be able to re-initialize and re-schedule all systems when a segment is added.
- This will happen when the 'Defer' module is resolved which occurs at the next synchronization point.
- There should not be a significant performance hit since segment addition/removal is expected to be rare and happen mainly
in the first frames of execution.

RESOURCES
- There will be 1 segment per resource such that the same segment/dependency system can be used for them.
- Resource segments should only allocate 1 store with 1 slot with the resource in it.
- Resource entities must not be query-able (could be accomplished with a simple 'bool' in segments).

DEPENDENCIES
- Design a contract API that ensures that dependencies are well defined.
- To gain access to a given resource, a user must provide a corresponding 'Contract' that is provided by a 'Contractor'.
- The 'Contractor' then stores a copy of each emitted contract to later convert them into corresponding dependencies.
- Ex: System::initialize(contractor: &mut Contractor, world: &mut World) -> Store<Time> {
    world.get_resource(contractor.resource(TypeId::of::<Time>()))
        OR
    world.get_resource::<Time>(contractor)
        OR
    contractor.resource::<Time>(world) // This doesn't require the 'World' to know about the 'Contractor'.
        OR
    contractor.resource::<Time>() // The contractor can hold its own reference to the 'World'.
}
*/

pub struct Template<T>(T);

#[derive(Default)]
pub(crate) struct Datum {
    index: u32,
    segment: u32,
}

pub struct World {
    pub(crate) free: Mutex<Vec<Entity>>,
    pub(crate) last: AtomicUsize,
    pub(crate) capacity: AtomicUsize,
    pub(crate) data: Vec<Datum>,
    pub(crate) segments: Vec<Segment>,
}

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) indices: HashMap<TypeId, usize>,
    pub(crate) stores: Box<[Pin<Box<dyn Any>>]>,
}

pub struct Store<T>(pub(crate) UnsafeCell<Box<[T]>>);

impl World {
    pub fn new() -> Self {
        Self {
            free: Vec::new().into(),
            last: 0.into(),
            capacity: 0.into(),
            data: Vec::new(),
            segments: Vec::new(),
        }
    }

    pub fn scheduler(&self) -> Scheduler {
        Scheduler {
            schedules: Vec::new(),
            world: self,
        }
    }

    pub fn segment(&mut self, types: &[TypeId]) -> &Segment {
        let segments = &self.segments;
        let mut index = segments.len();
        for i in 0..segments.len() {
            let segment = &segments[i];
            if segment.stores.len() == types.len()
                && types
                    .iter()
                    .all(|value| segment.indices.contains_key(value))
            {
                index = i;
                break;
            }
        }

        // if index == segments.len() {
        //     segments.push(Segment::default());
        // }
        &segments[index]
    }

    pub fn reserve(&self, _entities: &mut [Entity]) {
        // let mut a = <[Entity; 32]>::default();
        // self.reserve(&mut a);
        // let mut b = vec![Entity::default(); 32];
    }
}

impl Segment {
    pub fn store<T: Send + 'static>(&self) -> Option<&Store<T>> {
        let index = self.indices.get(&TypeId::of::<T>())?;
        self.stores[*index].downcast_ref()
    }
}

impl<T> Store<T> {
    pub fn new(capacity: usize) -> Self {
        let mut content: Vec<T> = Vec::with_capacity(capacity);
        unsafe { content.set_len(capacity) };
        Self(content.into_boxed_slice().into())
    }

    #[inline]
    pub unsafe fn at(&self, index: usize) -> &mut T {
        &mut self.get()[index]
    }

    #[inline]
    pub unsafe fn get(&self) -> &mut [T] {
        (&mut *self.0.get()).as_mut()
    }

    pub unsafe fn count(&self) -> usize {
        self.get().len()
    }
}
