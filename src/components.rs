use crate::world::Downcast;
use crate::*;
use once_cell::sync::Lazy;
use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Copy, Clone, Debug)]
pub struct Metadata {
    pub identifier: TypeId,
    pub name: &'static str,
    pub index: usize,
}

static REGISTRY: Lazy<Mutex<Option<Vec<Metadata>>>> = Lazy::new(|| Mutex::new(Some(Vec::new())));

impl Metadata {
    /// While not strictly unsafe, it is assumed that all calls to 'new' are done before any call
    /// to the 'get' functions and any further calls will cause a panic.
    pub unsafe fn new<T: Component + 'static>() -> Metadata {
        let mut guard = REGISTRY.lock().unwrap();
        let registry = guard.as_mut().unwrap();
        let meta = Metadata {
            identifier: TypeId::of::<T>(),
            name: type_name::<T>(),
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
    pub fn get_all() -> &'static Vec<Metadata> {
        static CACHE: Lazy<Vec<Metadata>> = Lazy::new(|| REGISTRY.lock().unwrap().take().unwrap());
        &CACHE
    }
}

pub(crate) trait Store {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> Store for Vec<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Downcast for dyn Store {
    fn cast<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }
}

pub(crate) struct Segment {
    pub stores: Vec<Box<dyn Store>>,
}

pub trait Component {
    fn metadata() -> &'static Metadata;
}

pub trait Components {
    fn has<T: Component + 'static>(&self, entity: Entity) -> bool;
    fn get<T: Component + 'static>(&self, entity: Entity) -> Option<&T>;
    fn set<T: Component + 'static>(&mut self, entity: Entity, component: T) -> bool;
    fn remove<T: Component + 'static>(&mut self, entity: Entity) -> bool;
}

impl World {
    #[inline]
    fn get_store<'a, T: Component + 'static>(&self, segment: &'a Segment) -> Option<&'a Vec<T>> {
        segment
            .stores
            .get(T::metadata().index)
            .and_then(|store| store.cast::<Vec<T>>())
    }
}

impl Components for World {
    fn has<T: Component + 'static>(&self, entity: Entity) -> bool {
        self.get_data(entity)
            .and_then(|data| self.get_store::<T>(&self.segments[data.segment as usize]))
            .is_some()
    }

    fn get<T: Component + 'static>(&self, entity: Entity) -> Option<&T> {
        self.get_data(entity).and_then(|data| {
            self.get_store::<T>(&self.segments[data.segment as usize])
                .map(|store| &store[data.index as usize])
        })
    }

    fn set<T: Component + 'static>(&mut self, entity: Entity, component: T) -> bool {
        todo!()
    }

    fn remove<T: Component + 'static>(&mut self, entity: Entity) -> bool {
        todo!()
    }
}
