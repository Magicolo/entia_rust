use entia_core::bits::Bits;
use std::any::type_name;
use std::any::TypeId;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::ptr::copy;
use std::ptr::drop_in_place;
use std::ptr::slice_from_raw_parts_mut;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::depend::Depend;
use crate::{
    depend::Dependency,
    inject::{Get, Inject},
    segment::Segment,
};

#[derive(Clone)]
pub struct Meta {
    pub(crate) identifier: TypeId,
    pub(crate) name: &'static str,
    pub(crate) index: usize,
    pub(crate) allocate: fn(usize) -> NonNull<()>,
    pub(crate) free: fn(NonNull<()>, usize),
    pub(crate) copy: fn((NonNull<()>, usize), (NonNull<()>, usize), usize),
    pub(crate) drop: fn(NonNull<()>, usize, usize),
}

pub struct World {
    pub(crate) identifier: usize,
    pub(crate) segments: Vec<Segment>,
    pub(crate) metas: Vec<Arc<Meta>>,
    pub(crate) type_to_meta: HashMap<TypeId, usize>,
    pub(crate) bits_to_segment: HashMap<Bits, usize>,
}

pub struct State;

impl Inject for &World {
    type Input = ();
    type State = State;

    fn initialize(_: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(State)
    }
}

impl<'a> Get<'a> for State {
    type Item = &'a World;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        world
    }
}

impl Depend for State {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

impl World {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Self {
            identifier: COUNTER.fetch_add(1, Ordering::Relaxed),
            segments: Vec::new(),
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
            bits_to_segment: HashMap::new(),
        }
    }

    pub fn get_meta<T: Send + 'static>(&self) -> Option<Arc<Meta>> {
        let key = TypeId::of::<T>();
        self.type_to_meta
            .get(&key)
            .map(|&index| self.metas[index].clone())
    }

    pub fn get_or_add_meta<T: Send + 'static>(&mut self) -> Arc<Meta> {
        match self.get_meta::<T>() {
            Some(meta) => meta,
            None => {
                let key = TypeId::of::<T>();
                let index = self.metas.len();
                let meta = Arc::new(Meta {
                    identifier: key.clone(),
                    name: type_name::<T>(),
                    index,
                    allocate: |capacity| unsafe {
                        let mut data = ManuallyDrop::new(Vec::<T>::with_capacity(capacity));
                        NonNull::new_unchecked(data.as_mut_ptr()).cast()
                    },
                    free: |pointer, capacity| unsafe {
                        Vec::from_raw_parts(pointer.cast::<T>().as_ptr(), 0, capacity);
                    },
                    copy: |source, target, count| unsafe {
                        let source = source.0.cast::<T>().as_ptr().add(source.1);
                        let target = target.0.cast::<T>().as_ptr().add(target.1);
                        copy(source, target, count);
                    },
                    drop: |pointer, index, count| unsafe {
                        let pointer = pointer.cast::<T>().as_ptr().add(index);
                        drop_in_place(slice_from_raw_parts_mut(pointer, count));
                    },
                });
                self.metas.push(meta.clone());
                self.type_to_meta.insert(key, index);
                meta
            }
        }
    }

    pub fn get_metas_from_types(&self, types: &Bits) -> Vec<Arc<Meta>> {
        types
            .into_iter()
            .map(|index| self.metas[index].clone())
            .collect()
    }

    pub fn get_segment_by_types(&self, types: &Bits) -> Option<&Segment> {
        self.bits_to_segment
            .get(types)
            .map(|&index| &self.segments[index])
    }

    pub fn get_segment_by_metas(&self, metas: Vec<Arc<Meta>>) -> Option<&Segment> {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_segment_by_types(&types)
    }

    pub fn add_segment_from_types(&mut self, types: &Bits, capacity: usize) -> &mut Segment {
        let metas = self.get_metas_from_types(types);
        self.add_segment(types.clone(), metas, capacity)
    }

    pub fn add_segment_from_metas(
        &mut self,
        mut metas: Vec<Arc<Meta>>,
        capacity: usize,
    ) -> &mut Segment {
        let mut types = Bits::new();
        metas.sort_by_key(|meta| meta.index);
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.add_segment(types, metas, capacity)
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
        metas: Vec<Arc<Meta>>,
        capacity: Option<usize>,
    ) -> &mut Segment {
        let mut types = Bits::new();
        metas.iter().for_each(|meta| types.set(meta.index, true));
        self.get_or_add_segment_by_types(&types, capacity)
    }

    fn add_segment(
        &mut self,
        types: Bits,
        metas: impl IntoIterator<Item = Arc<Meta>>,
        capacity: usize,
    ) -> &mut Segment {
        let index = self.segments.len();
        let segment = Segment::new(index, types.clone(), metas, capacity);
        self.segments.push(segment);
        self.bits_to_segment.insert(types, index);
        &mut self.segments[index]
    }
}
