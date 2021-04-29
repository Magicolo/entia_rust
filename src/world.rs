use std::any::Any;
use std::any::TypeId;
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
pub struct WorldInner {
    _entities: AtomicPtr<Entity>,
    _last: AtomicUsize,
    pub(crate) segments: Vec<Segment>,
}
#[derive(Default)]
pub struct World {
    pub(crate) inner: Arc<WorldInner>,
}

pub struct SegmentInner {
    pub index: usize,
    pub types: Vec<TypeId>,
    pub entities: Vec<Entity>,
    pub stores: Vec<Arc<dyn Any + Send + Sync + 'static>>,
    pub indices: HashMap<TypeId, usize>,
}
pub struct Segment {
    pub(crate) inner: Arc<SegmentInner>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_segment(&self, _types: &[TypeId]) -> Option<Segment> {
        todo!()
    }
}
