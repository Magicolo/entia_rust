use crate::entity::*;
use crate::system::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Template<T>(T);

pub struct Datum {
    pub(crate) index: u32,
    pub(crate) store: Arc<Store<Entity>>,
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
    metas: HashMap<TypeId, Meta>,
}

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: AtomicUsize,
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

    pub fn get_segment(&self, metas: &[Meta]) -> Option<Arc<Segment>> {
        self.get_segments(metas).next()
    }

    pub fn get_segments<'a>(
        &'a self,
        metas: &'a [Meta],
    ) -> impl Iterator<Item = Arc<Segment>> + 'a {
        self.segments
            .iter()
            .filter(move |segment| segment.is(metas))
            .cloned()
    }

    pub fn add_segment(&mut self, metas: &[Meta], capacity: usize) -> Arc<Segment> {
        let segment = Arc::new(Segment {
            index: self.segments.len(),
            count: 0.into(),
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
        self.indices.len() == metas.len() && self.has(metas)
    }

    pub fn has(&self, metas: &[Meta]) -> bool {
        metas
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
