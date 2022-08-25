use crate::{
    meta::{Meta, Metas},
    resource::Resource,
    store::Store,
};
use std::{any::TypeId, collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct Resources(HashMap<TypeId, Arc<Store>>);

impl Resources {
    pub fn get<R: Resource>(&self) -> Option<&R> {
        self.0
            .get(&TypeId::of::<R>())
            .map(|store| unsafe { &*store.get(0) })
    }

    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.0
            .get(&TypeId::of::<R>())
            .map(|store| unsafe { store.get(0) })
    }

    pub fn set<R: Resource>(&mut self, resource: R) -> Option<R> {
        match self.0.get(&TypeId::of::<R>()) {
            Some(store) => Some(unsafe { store.replace(0, resource) }),
            None => {
                let meta = self.with_metas(|metas| metas.get_or_add::<R>(R::meta));
                self.add_store(resource, meta);
                None
            }
        }
    }

    pub(crate) unsafe fn get_store<R: Resource, F: FnOnce() -> R>(
        &mut self,
        initialize: F,
    ) -> Arc<Store> {
        match self.0.get(&TypeId::of::<R>()) {
            Some(store) => store.clone(),
            None => {
                let meta = self.with_metas(|metas| metas.get_or_add::<R>(R::meta));
                self.add_store(initialize(), meta)
            }
        }
    }

    fn with_metas<T>(&mut self, map: impl FnOnce(&mut Metas) -> T) -> T {
        match self.0.get(&TypeId::of::<Metas>()) {
            Some(store) => map(unsafe { store.get(0) }),
            None => {
                let mut metas = Metas::default();
                let meta = metas.get_or_add::<Metas>(Metas::meta);
                let store = self.add_store(metas, meta);
                map(unsafe { store.get(0) })
            }
        }
    }

    fn add_store<T: Send + Sync + 'static>(&mut self, value: T, meta: Arc<Meta>) -> Arc<Store> {
        assert!(meta.is::<T>());
        let store = Arc::new(unsafe { Store::new(meta, 1) });
        unsafe { store.set(0, value) };
        self.0.insert(TypeId::of::<T>(), store.clone());
        store
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        for (_, store) in self.0.drain() {
            unsafe { store.free(1, 1) };
        }
    }
}
