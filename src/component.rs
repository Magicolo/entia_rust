use crate::world::Inner;
use crate::*;
use once_cell::sync::Lazy;
use std::any;
use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

pub type CreateStore = dyn Fn() -> Box<dyn Store> + Sync;

#[derive(Copy, Clone)]
pub struct Metadata {
    pub identifier: TypeId,
    pub name: &'static str,
    pub index: usize,
    store: &'static CreateStore,
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
            store: &|| Box::new(Vec::<C>::new()),
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

    pub fn new_store(&self) -> Box<dyn Store> {
        (*self.store)()
    }
}

pub trait Store {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Swaps the element at `index` with the last element and moves the swapped
    /// element to the end of the `target` store.
    fn swap_to(&mut self, target: &mut dyn Store, index: usize);
}

impl<T: 'static> Store for Vec<T> {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn swap_to(&mut self, target: &mut dyn Store, index: usize) {
        if let Some(store) = target.as_any_mut().downcast_mut::<Self>() {
            store.push(self.swap_remove(index));
        }
    }
}

pub struct Segment {
    pub index: usize,
    pub types: Vec<Metadata>,
    pub entities: Vec<Entity>,
    pub stores: Vec<Rc<dyn Any>>,
    pub storez: Vec<(*mut (), usize, usize)>,
}

pub trait Component {
    fn metadata() -> &'static Metadata;
}

impl Segment {
    #[inline]
    pub fn get_store<C: Component + 'static>(&self) -> Option<Rc<UnsafeCell<Vec<C>>>> {
        self.stores
            .get(C::metadata().index)
            .and_then(|store| store.clone().downcast::<UnsafeCell<Vec<C>>>().ok())
    }
}

impl Inner {
    pub fn has_component<C: Component + 'static>(&self, entity: Entity) -> bool {
        Self::get_entity_data(&self.data, entity)
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
