use crate::core::utility::*;
use crate::entity::*;
use crate::inject::*;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Datum {
    pub(crate) index: u32,
    pub(crate) segment: u32,
    pub(crate) store: Arc<Store<Entity>>,
}

#[derive(Clone)]
pub struct Meta {
    identifier: TypeId,
    store: fn(usize) -> (Arc<dyn Storage>, Arc<dyn Any + Send + Sync>),
}

pub trait Storage: Sync + Send {
    fn ensure(&self, capacity: usize) -> bool;
    fn copy_to(&self, source: usize, target: (usize, &dyn Any), count: usize) -> bool;
    fn copy(&self, source: usize, target: usize, count: usize) -> bool;
}

impl<T: Send + 'static> Storage for Store<T> {
    fn ensure(&self, capacity: usize) -> bool {
        let content = unsafe { self.get() };
        if content.len() < capacity {
            content.reserve(capacity - content.len());
            unsafe { content.set_len(capacity) };
            true
        } else {
            false
        }
    }

    fn copy_to(&self, source: usize, target: (usize, &dyn Any), count: usize) -> bool {
        if let Some(store) = target.1.downcast_ref::<Store<T>>() {
            unsafe { std::ptr::copy_nonoverlapping(self.at(source), store.at(target.0), count) };
            true
        } else {
            false
        }
    }

    fn copy(&self, source: usize, target: usize, count: usize) -> bool {
        if source == target {
            false
        } else {
            unsafe { std::ptr::copy_nonoverlapping(self.at(source), self.at(target), count) };
            true
        }
    }
}

/*
Create<(Position, Velocity, Option<Mass>)>(candidates)
- The filter selects at most 2 segments ([Position, Velocity, Mass], [Position, Velocity]) and selects the appropriate one
base on the provided 'Mass'.
- This means that 'Create' has an 'Add' dependency on these 2 segments.
- When calling 'Create::create<const N: usize>(self, initialize) -> [Entity; N]':
    let segment = initialize.select_candidate(candidates);
    let count = segment.count.fetch_add(N);
    let index = count - N;
    if count <= segment.capacity {
        initialize.initialize(segment.index, index, count);
    } else {
        self.defer(initialize, segment.index, index, count);
    }
- 'Create' can be concurrent to 'Read/Write' within its candidate segments as long as the dependency appears after the 'Read/Write'.
- 'Create' is incompatible with 'Destroy' when candidates overlap
- 'Create' is compatible with 'Add(targets)' but not with 'Add(sources)'
- 'Create' is compatible with 'Remove(targets)' but not with 'Remove(sources)'

Destroy<(Position, Velocity)>(candidates)




- Has a 'Write' dependency on all segments that have at least [Position, Velocity].
- Has a 'Add' dependency on all segments that

Remove<(Position, Velocity)>({ source: [target] })

fn try_add(segment: &segment, initialize: impl Initialize) {
    let count = segment.count.fetch_add(some_amount);
    let index = count - some_amount;
    if count < segment.capacity {
        // write to the store
        initialize.initialize(segment, index, some_amount);
    } else {
        // defer the 'resize' and the 'write'
        defer.initialize(segment.index, index, some_amount, initialize);
    }
}

let index = count - some_amount;
loop {
    let capacity = segment.capacity.load();
    if count <= capacity {
        break;
    }

    // This doesn't work since there might be live read/write pointers the store...
    let guard = segment.lock.lock();
    let capacity = next_power_of_2(count);
    segment.stores.iter().for_each(|store| store.ensure(capacity));
    segment.capacity.fetch_max(capacity);
}
*/
pub struct Segment {
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: AtomicUsize,
    pub(crate) indices: HashMap<TypeId, usize>,
    pub(crate) stores: Box<[(Meta, Arc<dyn Storage>, Arc<dyn Any + Send + Sync>)]>,
}

pub struct Store<T>(pub(crate) UnsafeCell<Vec<T>>);

pub struct World {
    pub(crate) identifier: usize,
    pub(crate) segments: Vec<Segment>,
    metas: HashMap<TypeId, Meta>,
}

pub struct WorldState;
impl Inject for &World {
    type Input = ();
    type State = WorldState;

    fn initialize(_: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(WorldState)
    }
}

impl<'a> Get<'a> for WorldState {
    type Item = &'a World;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        world
    }
}

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
                    store: |capacity| {
                        let store = Arc::new(unsafe { Store::<T>::new(capacity) });
                        (store.clone(), store)
                    },
                };
                self.metas.insert(key, meta.clone());
                meta
            }
        }
    }

    pub fn get_segment(&self, metas: &[Meta]) -> Option<&Segment> {
        for segment in self.segments.iter() {
            if segment.is(metas) {
                return Some(segment);
            }
        }
        None
    }

    pub fn get_segments<'a>(&'a self, metas: &'a [Meta]) -> impl Iterator<Item = &'a Segment> + 'a {
        self.segments
            .iter()
            .filter(move |segment| segment.is(metas))
    }

    pub fn add_segment(&mut self, metas: &[Meta], capacity: usize) -> &mut Segment {
        let index = self.segments.len();
        self.segments.push(Segment {
            index,
            count: 0.into(),
            capacity,
            indices: metas
                .iter()
                .enumerate()
                .map(|pair| (pair.1.identifier.clone(), pair.0))
                .collect(),
            stores: metas
                .iter()
                .map(|meta| {
                    let stores = (meta.store)(capacity);
                    (meta.clone(), stores.0, stores.1)
                })
                .collect(),
        });
        &mut self.segments[index]
    }

    pub fn get_or_add_segment(&mut self, metas: &[Meta], capacity: Option<usize>) -> &mut Segment {
        match self.get_segment(metas).map(|segment| segment.index) {
            Some(index) => {
                let segment = &mut self.segments[index];
                if let Some(capacity) = capacity {
                    segment.ensure(capacity);
                }
                segment
            }
            None => self.add_segment(metas, capacity.unwrap_or(32)),
        }
    }
}

impl Segment {
    pub fn move_to(&mut self, index: usize, target: &mut Segment) -> Option<usize> {
        if self.index == target.index {
            return None;
        }

        let source_count = self.count.get_mut();
        *source_count -= 1;
        let source_last = *source_count;

        let target_count = target.count.get_mut();
        let target_last = *target_count;
        *target_count += 1;
        let target_count = *target_count;
        target.ensure(target_count);

        for (meta, source, _) in self.stores.iter() {
            if let Some((_, _, target)) = target
                .indices
                .get(&meta.identifier)
                .map(|&index| &target.stores[index])
            {
                source.copy_to(index, (target_last, target.as_ref()), 1);
                source.copy(source_last, index, 1);
            }
        }

        Some(target_last)
    }

    pub fn store<T: Send + 'static>(&self) -> Option<Arc<Store<T>>> {
        let index = self.indices.get(&TypeId::of::<T>())?;
        self.stores[*index].2.clone().downcast().ok()
    }

    pub fn is(&self, metas: &[Meta]) -> bool {
        self.indices.len() == metas.len() && self.has(metas)
    }

    pub fn has(&self, metas: &[Meta]) -> bool {
        metas
            .iter()
            .all(|meta| self.indices.contains_key(&meta.identifier))
    }

    pub fn ensure(&mut self, capacity: usize) -> bool {
        if self.capacity <= capacity {
            false
        } else {
            self.capacity = next_power_of_2(capacity as u32) as usize;
            for (_, store, _) in self.stores.iter_mut() {
                store.ensure(self.capacity);
            }
            true
        }
    }
}

impl<T> Store<T> {
    pub unsafe fn new(capacity: usize) -> Self {
        let mut content: Vec<T> = Vec::with_capacity(capacity);
        content.set_len(capacity);
        Self(content.into())
    }

    #[inline]
    pub unsafe fn at(&self, index: usize) -> &mut T {
        &mut self.get()[index]
    }

    #[inline]
    pub unsafe fn get(&self) -> &mut Vec<T> {
        (&mut *self.0.get()).as_mut()
    }
}
