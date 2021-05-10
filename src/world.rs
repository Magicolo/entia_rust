use crate::entity::*;
use crate::initialize::*;
use crate::system::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Template<T>(T);

pub struct Datum {
    pub(crate) index: u32,
    pub(crate) segment: Arc<Segment>,
}

#[derive(Clone)]
pub struct Meta {
    identifier: TypeId,
    store: fn(usize) -> Arc<dyn Any + Sync + Send>,
}

pub struct World {
    pub(crate) identifier: usize,
    pub(crate) segments: Vec<Arc<Segment>>,
    free: Vec<Entity>,
    last: AtomicU32,
    capacity: AtomicUsize,
    data: Vec<Datum>,
    metas: HashMap<TypeId, Meta>,
}

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) capacity: usize,
    pub(crate) indices: HashMap<TypeId, usize>,
    pub(crate) entities: Arc<Store<Entity>>,
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

    pub fn create_entity(&mut self, initialize: impl Initialize) -> (Entity, Arc<Segment>) {
        let (entities, segment): ([_; 1], _) = self.create_entities(initialize);
        (entities[0], segment)
    }

    pub fn create_entities<const N: usize>(
        &mut self,
        initialize: impl Initialize,
    ) -> ([Entity; N], Arc<Segment>) {
        let entities = self.reserve_entities();
        let segment = self.initialize_entities(&entities, initialize);
        (entities, segment)
    }

    pub fn reserve_entities<const N: usize>(&self) -> [Entity; N] {
        // TODO: use 'MaybeUninit'?
        let mut entities = [Entity::default(); N];
        entities
    }

    pub fn initialize_entities<I: Initialize>(
        &mut self,
        entities: &[Entity],
        initialize: I,
    ) -> Arc<Segment> {
        let metas = I::metas(self);
        let segment = self.get_or_add_segment(&metas, None);
        todo!();
        segment
    }

    pub fn has_entity(&self, entity: Entity) -> bool {
        match self.get_datum(entity) {
            Some(datum) => *unsafe { datum.segment.entities.at(datum.index as usize) } == entity,
            None => false,
        }
    }

    pub fn destroy_entities(&mut self, entities: &[Entity]) -> usize {
        todo!()
    }

    pub fn get_meta<T: Send + 'static>(&self) -> Option<Meta> {
        let key = TypeId::of::<T>();
        self.metas.get(&key).cloned()
    }

    pub fn get_or_add_meta<T: Send + 'static>(&mut self) -> Meta {
        match self.get_meta::<T>() {
            Some(meta) => meta,
            None => {
                let key = TypeId::of::<T>();
                let meta = Meta {
                    identifier: key.clone(),
                    store: |capacity| Arc::new(Store::<T>::new(capacity)),
                };
                self.metas.insert(key, meta.clone());
                meta
            }
        }
    }

    #[inline]
    pub fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        if entity.index < self.last.load(Ordering::Relaxed) {
            Some(&self.data[entity.index as usize])
        } else {
            None
        }
    }

    pub fn get_segment(&self, metas: &[Meta]) -> Option<Arc<Segment>> {
        for segment in self.segments.iter() {
            if segment.is(metas) {
                return Some(segment.clone());
            }
        }
        None
    }

    pub fn add_segment(&mut self, metas: &[Meta], capacity: usize) -> Arc<Segment> {
        let segment = Arc::new(Segment {
            index: self.segments.len(),
            count: 0,
            capacity,
            indices: metas
                .iter()
                .enumerate()
                .map(|pair| (pair.1.identifier.clone(), pair.0))
                .collect(),
            entities: Store::new(capacity).into(),
            stores: metas
                .iter()
                .map(|meta| (meta.clone(), (meta.store)(capacity)))
                .collect(),
        });
        self.segments.push(segment.clone());
        segment
    }

    pub fn get_or_add_segment(&mut self, metas: &[Meta], capacity: Option<usize>) -> Arc<Segment> {
        match self.get_segment(metas) {
            Some(segment) => {
                let capacity = capacity.unwrap_or(segment.capacity);
                segment.ensure(capacity);
                segment
            }
            None => self.add_segment(metas, capacity.unwrap_or(32)),
        }
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

    pub fn ensure(&self, capacity: usize) -> bool {
        if self.capacity <= capacity {
            false
        } else {
            // TODO: Resize stores.
            true
        }
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
