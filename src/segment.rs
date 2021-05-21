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

pub struct Move(usize, usize, Vec<(Arc<dyn Storage>, Arc<dyn Any>)>);

impl Move {
    pub fn apply(&self, index: usize, world: &mut World) -> Option<usize> {
        if self.0 == self.1 {
            Some(index)
        } else if let Some((source, target)) = get_mut2(&mut world.segments, (self.0, self.1)) {
            source.count -= 1;
            let source_index = source.count;
            let target_index = target.reserve(1);
            for (source_store, target_store) in &self.2 {
                source_store.copy_to(index, (target_index, target_store.as_ref()), 1);
                source_store.copy(source_index, index, 1);
            }
            Some(target_index)
        } else {
            None
        }
    }

    #[inline]
    pub const fn source(&self) -> usize {
        self.0
    }

    #[inline]
    pub const fn target(&self) -> usize {
        self.1
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
        let mut stores = Vec::new();
        for (meta, source, _) in self.stores.iter() {
            if let Some(target) = target.dynamic_store(meta) {
                stores.push((source.clone(), target));
            }
        }
        Move(self.index, target.index, stores)
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
