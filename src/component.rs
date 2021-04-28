use crate::*;
use once_cell::sync::Lazy;
use std::any;
use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Copy, Clone)]
pub struct Metadata {
    pub identifier: TypeId,
    pub name: &'static str,
    pub index: usize,
}

static REGISTRY: Lazy<Mutex<Option<Vec<Metadata>>>> = Lazy::new(|| Mutex::new(Some(Vec::new())));

impl Metadata {
    /// It is assumed that all calls to 'new' are done before any call to the 'get' functions.
    /// Any further calls will cause a panic.
    pub unsafe fn new<C: Component + 'static>() -> Metadata {
        let mut guard = REGISTRY.lock().unwrap();
        let registry = guard.as_mut().unwrap();
        let meta = Metadata {
            identifier: TypeId::of::<C>(),
            name: any::type_name::<C>(),
            index: registry.len(),
        };
        registry.push(meta);
        meta
    }

    #[inline]
    pub fn get_by_index(index: usize) -> Option<&'static Metadata> {
        Metadata::get_all().get(index)
    }

    #[inline]
    pub fn get_by_name(name: &str) -> Option<&'static Metadata> {
        static CACHE: Lazy<HashMap<&'static str, &'static Metadata>> = Lazy::new(|| {
            Metadata::get_all()
                .iter()
                .map(|meta| (meta.name, meta))
                .collect()
        });

        CACHE.get(name).map(|meta| *meta)
    }

    #[inline]
    pub fn get_by_type(identifier: &TypeId) -> Option<&'static Metadata> {
        static CACHE: Lazy<HashMap<TypeId, &'static Metadata>> = Lazy::new(|| {
            Metadata::get_all()
                .iter()
                .map(|meta| (meta.identifier, meta))
                .collect()
        });

        CACHE.get(identifier).map(|meta| *meta)
    }

    #[inline]
    pub fn get_all() -> &'static [Metadata] {
        static CACHE: Lazy<Vec<Metadata>> = Lazy::new(|| REGISTRY.lock().unwrap().take().unwrap());
        &CACHE
    }

    // pub fn new_store(&self) -> Box<dyn Store> {
    //     (*self.store)()
    // }
}

pub(crate) struct Inner {
    pub index: usize,
    pub types: Vec<Metadata>,
    pub entities: Vec<Entity>,
    pub stores: Vec<Box<dyn Any>>,
}

#[derive(Clone)]
pub struct Segment {
    inner: Arc<UnsafeCell<Inner>>,
}

// #[derive(Clone)]
pub struct Store<T> {
    pub(crate) inner: Arc<UnsafeCell<Vec<T>>>,
}
impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        Store {
            inner: self.inner.clone(),
        }
    }
}

pub trait Component {}

impl Segment {
    #[inline]
    pub fn get_store<T: 'static>(&self) -> Option<&Store<T>> {
        todo!()
        // self.stores
        //     .get(C::metadata().index)
        //     .and_then(|store| store.clone().downcast::<UnsafeCell<Vec<C>>>().ok())
    }

    #[inline]
    pub(crate) unsafe fn get(&self) -> &mut Inner {
        &mut *self.inner.get()
    }
}

impl<T> Store<T> {
    #[inline]
    pub(crate) unsafe fn get(&self) -> &mut Vec<T> {
        &mut *self.inner.get()
    }
}

impl Inner {
    fn get_store<T: 'static>(&mut self) -> Option<&mut Vec<T>> {
        todo!()
    }

    fn get_store_at<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.get_store().and_then(|store| store.get_mut(index))
    }
}

impl world::Inner {
    pub fn has_component<C: Component + 'static>(&self, entity: Entity) -> bool {
        self.get_entity_data(entity)
            .and_then(|data| self.segments[data.segment as usize].get_store::<C>())
            .is_some()
    }

    pub fn get_component<C: Component + 'static>(&mut self, entity: Entity) -> Option<&mut C> {
        if let Some(data) = Self::get_data_mut(&mut self.data, entity) {
            if let Some(store) = self.segments[data.segment as usize].get_store() {
                let store = unsafe { &mut *store.get() };
                return Some(&mut store[data.index as usize]);
            }
        }
        None
        // self.get_data_mut(entity).and_then(|data| {
        //     self.segments[data.segment as usize]
        //         .get_store()
        //         .map(|store| {
        //             let store = unsafe { &mut *store.get() };
        //             &mut store[data.index as usize]
        //         })
        // })
    }

    pub fn set_component<C: Component + 'static>(&mut self, _: Entity, _: C) -> bool {
        todo!()
    }

    pub fn remove_component<C: Component + 'static>(&mut self, _: Entity) -> bool {
        todo!()
    }
}
