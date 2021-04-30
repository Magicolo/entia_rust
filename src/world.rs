use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
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

#[derive(Default)]
pub struct World {
    pub(crate) free: Mutex<Vec<Entity>>,
    pub(crate) last: AtomicUsize,
    pub(crate) capacity: AtomicUsize,
    pub(crate) data: Store<Datum>,
    pub(crate) segments: Vec<Segment>,
}

#[derive(Default)]
pub struct Segment {
    pub(crate) index: usize,
    pub(crate) entities: Arc<Store<Entity>>,
    pub(crate) stores: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
}

#[derive(Default)]
pub struct Store<T>(pub(crate) UnsafeCell<Vec<T>>);
unsafe impl<T> Sync for Store<T> {}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn segment(&mut self, types: &[TypeId]) -> &Segment {
        let mut index = self.segments.len();
        for i in 0..self.segments.len() {
            let segment = &self.segments[i];
            if segment.stores.len() == types.len()
                && types
                    .iter()
                    .all(|value| segment.stores.get(value).is_some())
            {
                index = i;
                break;
            }
        }

        if index == self.segments.len() {
            self.segments.push(Segment::default());
        }
        &self.segments[index]
    }

    pub fn reserve(&self, entities: &mut [Entity]) {
        // let mut a = <[Entity; 32]>::default();
        // self.reserve(&mut a);
        // let mut b = vec![Entity::default(); 32];
    }
}

impl Segment {
    pub fn store<T: Send + 'static>(&self) -> Option<Arc<Store<T>>> {
        self.stores.get(&TypeId::of::<T>())?.clone().downcast().ok()
    }
}

impl<T> Store<T> {
    // TODO: The "<'a>" is highly unsafe (could be resolved as 'static). Is there a less unsafe workaround?
    #[inline]
    pub unsafe fn at<'a>(&self, index: usize) -> &'a mut T {
        &mut self.get()[index]
    }

    #[inline]
    pub unsafe fn get<'a>(&self) -> &'a mut Vec<T> {
        &mut *self.0.get()
    }

    pub unsafe fn count(&self) -> usize {
        self.get().len()
    }
}
