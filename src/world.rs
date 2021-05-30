use crate::segment::*;
use crate::{inject::*, system::Dependency};
use entia_core::bits::Bits;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Clone)]
pub struct Meta {
    pub(crate) identifier: TypeId,
    pub(crate) index: usize,
    pub(crate) store: fn(usize) -> (Arc<dyn Storage>, Arc<dyn Any + Send + Sync>),
}

pub struct Store<T>(pub(crate) UnsafeCell<Vec<T>>);

pub struct World {
    pub(crate) identifier: usize,
    pub(crate) segments: Vec<Segment>,
    pub(crate) metas: Vec<Meta>,
    pub(crate) types: HashMap<TypeId, usize>,
    pub(crate) bits: HashMap<Bits, usize>,
}

pub struct State;

pub trait Storage: Sync + Send {
    fn ensure(&self, capacity: usize) -> bool;
    fn foreign_copy(&self, source: usize, target: (usize, &dyn Any), count: usize) -> bool;
    fn local_copy(&self, source: usize, target: usize, count: usize) -> bool;
    fn clear(&self, index: usize, count: usize);
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

    fn foreign_copy(&self, source: usize, target: (usize, &dyn Any), count: usize) -> bool {
        if let Some(store) = target.1.downcast_ref::<Store<T>>() {
            unsafe { std::ptr::copy_nonoverlapping(self.at(source), store.at(target.0), count) };
            true
        } else {
            false
        }
    }

    fn local_copy(&self, source: usize, target: usize, count: usize) -> bool {
        if source == target {
            false
        } else {
            unsafe { std::ptr::copy_nonoverlapping(self.at(source), self.at(target), count) };
            true
        }
    }

    fn clear(&self, index: usize, mut count: usize) {
        while count > 0 {
            count -= 1;
            drop(unsafe { self.at(index + count) });
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

impl Inject for &World {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(State)
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

impl<'a> Get<'a> for State {
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
            metas: Vec::new(),
            types: HashMap::new(),
            bits: HashMap::new(),
        }
    }

    pub fn get_meta<T: Send + 'static>(&self) -> Option<Meta> {
        let key = TypeId::of::<T>();
        self.types.get(&key).map(|&index| self.metas[index].clone())
    }

    pub fn get_or_add_meta<T: Send + 'static>(&mut self) -> Meta {
        match self.get_meta::<T>() {
            Some(meta) => meta,
            None => {
                let key = TypeId::of::<T>();
                let index = self.metas.len();
                let meta = Meta {
                    identifier: key.clone(),
                    index,
                    store: |capacity| {
                        let store = Arc::new(unsafe { Store::<T>::new(capacity) });
                        (store.clone(), store)
                    },
                };
                self.metas.push(meta.clone());
                self.types.insert(key, index);
                meta
            }
        }
    }

    pub fn get_metas_from_types(&self, types: &Bits) -> Vec<Meta> {
        types
            .into_iter()
            .map(|index| self.metas[index].clone())
            .collect()
    }

    pub fn get_segment_by_types(&self, types: &Bits) -> Option<&Segment> {
        self.bits.get(types).map(|&index| &self.segments[index])
    }

    pub fn get_segment_by_metas(&self, metas: &[Meta]) -> Option<&Segment> {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_segment_by_types(&types)
    }

    pub fn add_segment_from_types(&mut self, types: &Bits, capacity: usize) -> &mut Segment {
        let metas = self.get_metas_from_types(types);
        self.add_segment(types.clone(), &metas, capacity)
    }

    pub fn add_segment_from_metas(&mut self, metas: &[Meta], capacity: usize) -> &mut Segment {
        let mut metas: Box<[_]> = metas.iter().cloned().collect();
        let mut types = Bits::new();
        metas.sort_by_key(|meta| meta.index);
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.add_segment(types, &metas, capacity)
    }

    pub fn get_or_add_segment_by_types(
        &mut self,
        types: &Bits,
        capacity: Option<usize>,
    ) -> &mut Segment {
        match self
            .get_segment_by_types(types)
            .map(|segment| segment.index)
        {
            Some(index) => {
                let segment = &mut self.segments[index];
                if let Some(capacity) = capacity {
                    segment.ensure(capacity);
                }
                segment
            }
            None => self.add_segment_from_types(types, capacity.unwrap_or(32)),
        }
    }

    pub fn get_or_add_segment_by_metas(
        &mut self,
        metas: &[Meta],
        capacity: Option<usize>,
    ) -> &mut Segment {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_or_add_segment_by_types(&types, capacity)
    }

    fn add_segment(&mut self, types: Bits, metas: &[Meta], capacity: usize) -> &mut Segment {
        let index = self.segments.len();
        self.segments
            .push(Segment::new(index, types, &metas, capacity));
        &mut self.segments[index]
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
