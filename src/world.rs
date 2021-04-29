use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

pub trait Resource: Send + 'static {}
pub trait Component: Send + 'static {}

#[derive(Default)]
pub struct World {
    pub(crate) inner: Arc<WorldInner>,
}

pub struct Segment {
    pub(crate) inner: Arc<SegmentInner>,
}

#[derive(Default)]
pub struct WorldInner {
    pub entities: AtomicPtr<Entity>,
    pub last: AtomicUsize,
    pub segments: Vec<Segment>,
}

#[derive(Default)]
pub struct SegmentInner {
    pub index: usize,
    pub entities: Vec<Entity>,
    pub stores: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
}

pub struct Store<T>(UnsafeCell<Vec<T>>);
unsafe impl<T> Sync for Store<T> {}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_segment(&self, _types: &[TypeId]) -> Option<Segment> {
        todo!()
    }
}

impl Segment {
    pub fn store<T: Send + 'static>(&self) -> Option<Arc<Store<T>>> {
        self.inner
            .stores
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast()
            .ok()
    }
}

impl<T> Store<T> {
    // TODO: The "<'a>" is highly unsafe. Is there a less unsafe workaround?
    #[inline]
    pub unsafe fn get<'a>(&self, index: usize) -> &'a mut T {
        &mut (&mut *self.0.get())[index]
    }
}
