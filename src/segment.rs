use crate::core::bits::*;
use crate::core::utility::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::Arc;
use std::{any::Any, usize};

pub struct Segment {
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) capacity: usize,
    pub(crate) types: Bits,
    pub(crate) stores: Box<[(Meta, Arc<dyn Storage>, Arc<dyn Any + Send + Sync>)]>,
}

pub struct Move {
    source: usize,
    target: usize,
    copy: Vec<(Arc<dyn Storage>, Arc<dyn Any>)>,
    clear: Vec<Arc<dyn Storage>>,
}

unsafe impl Send for Move {}

impl Move {
    pub fn apply(&mut self, index: usize, count: usize, world: &mut World) -> Option<usize> {
        let indices = (self.source, self.target);
        if indices.0 == indices.1 {
            Some(index)
        } else if let Some((source, target)) = get_mut2(&mut world.segments, indices) {
            source.count -= count;
            let source_index = source.count;
            let target_index = target.reserve(count);
            for (source_store, target_store) in self.copy.iter() {
                source_store.foreign_copy(index, (target_index, target_store.as_ref()), count);
                source_store.local_copy(source_index, index, count);
            }

            for store in self.clear.iter() {
                store.clear(source_index, 1);
            }
            Some(target_index)
        } else {
            None
        }
    }

    #[inline]
    pub const fn source(&self) -> usize {
        self.source
    }

    #[inline]
    pub const fn target(&self) -> usize {
        self.target
    }
}

impl Segment {
    pub(crate) fn new(index: usize, types: Bits, metas: &[Meta], capacity: usize) -> Self {
        let stores: Box<[_]> = metas
            .iter()
            .map(|meta| {
                let stores = (meta.store)(capacity);
                (meta.clone(), stores.0, stores.1)
            })
            .collect();
        Self {
            index,
            count: 0,
            capacity,
            types,
            stores,
        }
    }

    pub fn prepare_move(&self, target: &Segment) -> Move {
        if self.index == target.index {
            Move {
                source: self.index,
                target: target.index,
                copy: Vec::new(),
                clear: Vec::new(),
            }
        } else {
            let mut copy = Vec::new();
            let mut clear = Vec::new();
            for (meta, source, _) in self.stores.iter() {
                if let Some(target) = target.dynamic_store(meta) {
                    copy.push((source.clone(), target));
                } else {
                    clear.push(source.clone())
                }
            }
            Move {
                source: self.index,
                target: target.index,
                copy,
                clear,
            }
        }
    }

    pub fn clear_at(&mut self, index: usize) -> bool {
        if index < self.count {
            self.count -= 1;
            let last = self.count;
            for (_, store, _) in self.stores.iter() {
                store.clear(index, 1);
                store.local_copy(last, index, 1);
            }
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        for (_, store, _) in self.stores.iter() {
            store.clear(0, self.count);
        }
        self.count = 0;
    }

    pub fn static_store<T: Send + 'static>(&self) -> Option<Arc<Store<T>>> {
        let identifier = TypeId::of::<T>();
        for (meta, _, store) in self.stores.iter() {
            if meta.identifier == identifier {
                return store.clone().downcast().ok();
            }
        }
        None
    }

    pub fn dynamic_store(&self, meta: &Meta) -> Option<Arc<dyn Any>> {
        if self.types.has(meta.index) {
            for store in self.stores.iter() {
                if store.0.index == meta.index {
                    return Some(store.2.clone());
                }
            }
        }
        None
    }

    pub fn reserve(&mut self, count: usize) -> usize {
        let index = self.count;
        self.count += count;
        self.ensure(self.count);
        index
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
