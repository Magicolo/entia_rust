use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
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

#[derive(Clone)]
pub struct Meta {
    identifier: TypeId,
    store: fn(usize) -> Arc<dyn Any + Sync + Send>,
}

pub struct World {
    pub(crate) identifier: usize,
    pub(crate) free: Mutex<Vec<Entity>>,
    pub(crate) last: AtomicUsize,
    pub(crate) capacity: AtomicUsize,
    pub(crate) data: Vec<Datum>,
    pub(crate) segments: Vec<Arc<Segment>>,
    metas: HashMap<TypeId, Meta>,
}

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) capacity: usize,
    pub(crate) indices: HashMap<TypeId, usize>,
    pub(crate) stores: Box<[(Meta, Arc<dyn Any + Sync + Send>)]>,
}

pub struct Store<T>(pub(crate) UnsafeCell<Box<[T]>>);
unsafe impl<T> Sync for Store<T> {}

impl World {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Self {
            identifier: COUNTER.fetch_add(1, Ordering::Relaxed),
            free: Vec::new().into(),
            last: 0.into(),
            capacity: 0.into(),
            data: Vec::new(),
            segments: Vec::new(),
            metas: HashMap::new(),
        }
    }

    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            schedules: Vec::new(),
            world: self,
        }
    }

    pub fn meta<T: Send + 'static>(&mut self) -> Meta {
        let key = TypeId::of::<T>();
        match self.metas.get(&key) {
            Some(meta) => meta.clone(),
            None => {
                let meta = Meta {
                    identifier: key.clone(),
                    store: |capacity| Arc::new(Store::<T>::new(capacity)),
                };
                self.metas.insert(key, meta.clone());
                meta
            }
        }
    }

    pub fn segment(&mut self, metas: &[Meta], capacity: Option<usize>) -> Arc<Segment> {
        for segment in self.segments.iter() {
            if segment.is(metas) {
                return segment.clone();
            }
        }

        let capacity = capacity.unwrap_or(32);
        let segment = Arc::new(Segment {
            index: self.segments.len(),
            count: 0,
            capacity,
            indices: metas
                .iter()
                .enumerate()
                .map(|pair| (pair.1.identifier.clone(), pair.0))
                .collect(),
            stores: metas
                .iter()
                .map(|meta| (meta.clone(), (meta.store)(capacity)))
                .collect(),
        });
        self.segments.push(segment.clone());
        segment
    }

    pub fn reserve(&self, _entities: &mut [Entity]) {
        // let mut a = <[Entity; 32]>::default();
        // self.reserve(&mut a);
        // let mut b = vec![Entity::default(); 32];
    }
}

impl Segment {
    pub fn store<T: Send + 'static>(&self) -> Option<Arc<Store<T>>> {
        let index = self.indices.get(&TypeId::of::<T>())?;
        self.stores[*index].1.clone().downcast().ok()
    }

    pub fn is(&self, metas: &[Meta]) -> bool {
        self.indices.len() == metas.len()
            && metas
                .iter()
                .all(|meta| self.indices.contains_key(&meta.identifier))
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
