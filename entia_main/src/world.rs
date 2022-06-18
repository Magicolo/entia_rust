use crate::{
    entity::Entity,
    error::{Error, Result},
    meta::Meta,
    segment::Segment,
    store::Store,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// Such a 'Link' would allow to compute which components have been added or removed.
/*
- Add 'Added/Removed<T>' query filters. The filters would hold a 'Bits' that represent the indices:
    fn dynamic_filter(state: &mut Self::State, index: usize) -> bool {
        state.bits.set(index, false) ????
    }
    - Will be equivalent to receiving a 'OnAdd<T>' message and 'query.get(onAdd.entity)'.
*/
// enum Link {
//     None,
//     Add { meta: usize, segment: usize },
//     Remove { meta: usize, segment: usize },
// }

pub struct World {
    identifier: usize,
    version: usize,
    metas: Vec<Arc<Meta>>,
    type_to_meta: HashMap<TypeId, usize>,
    resources: HashMap<TypeId, Arc<Store>>,
    // SAFETY: This vector may only 'push', never 'pop'; otherwise some unsafe index access may become invalid.
    segments: Vec<Segment>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            identifier: Self::reserve(),
            version: 1,
            metas: Vec::new(),
            type_to_meta: HashMap::new(),
            resources: HashMap::new(),
            segments: Vec::new(),
        };

        // Ensures that the 'Entity' meta has the lowest identifier of this world's metas and as such, 'Entity' stores will alway
        // appear as the first store of a segment if present.
        world.set_meta(crate::meta!(Entity));
        world
    }

    pub fn reserve() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub const fn version(&self) -> usize {
        self.version
    }

    #[inline]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    #[inline]
    pub fn segments_mut(&mut self) -> &mut [Segment] {
        &mut self.segments
    }

    pub fn set_meta(&mut self, meta: Meta) -> Arc<Meta> {
        let identifier = meta.identifier();
        let meta = match self.type_to_meta.get(&identifier) {
            Some(&index) => {
                let meta = Arc::new(meta);
                self.metas[index] = meta.clone();
                meta
            }
            None => {
                let index = self.metas.len();
                let meta = Arc::new(meta);
                self.metas.push(meta.clone());
                self.type_to_meta.insert(meta.identifier(), index);
                meta
            }
        };
        self.version += 1;
        meta
    }

    pub fn get_meta<T: Send + Sync + 'static>(&self) -> Result<Arc<Meta>> {
        self.get_meta_with(TypeId::of::<T>())
    }

    pub fn get_meta_with(&self, identifier: TypeId) -> Result<Arc<Meta>> {
        match self.type_to_meta.get(&identifier) {
            Some(&index) => Ok(self.metas[index].clone()),
            None => Err(Error::MissingMeta { identifier }),
        }
    }

    pub fn get_or_add_meta<T: Send + Sync + 'static, M: FnOnce() -> Meta>(
        &mut self,
        meta: M,
    ) -> Arc<Meta> {
        match self.get_meta::<T>() {
            Ok(meta) => meta,
            Err(_) => self.set_meta(meta()),
        }
    }

    pub fn get_segment(&self, metas: impl IntoIterator<Item = TypeId>) -> Option<&Segment> {
        Some(&self.segments[self.get_segment_index(&metas.into_iter().collect())?])
    }

    pub fn get_or_add_segment<'a, 'b>(
        &'a mut self,
        metas: impl IntoIterator<Item = impl Deref<Target = Meta>>,
    ) -> &'a mut Segment {
        self.get_or_add_segment_with(metas.into_iter().map(|meta| meta.identifier()))
    }

    pub fn get_or_add_segment_with<'a, 'b>(
        &'a mut self,
        types: impl IntoIterator<Item = TypeId>,
    ) -> &'a mut Segment {
        let types: HashSet<_> = types.into_iter().collect();
        let index = match self.get_segment_index(&types) {
            Some(index) => index,
            None => {
                let index = self.segments.len();
                let segment = Segment::new(Self::reserve(), index, 0, types, &self.metas);
                self.segments.push(segment);
                self.version += 1;
                index
            }
        };
        &mut self.segments[index]
    }

    pub(crate) fn get_or_add_resource_store<
        T: Send + Sync + 'static,
        M: FnOnce() -> Meta,
        I: FnOnce(&Meta, &mut World) -> Result<T>,
    >(
        &mut self,
        meta: M,
        initialize: I,
    ) -> Result<Arc<Store>> {
        let meta = self.get_or_add_meta::<T, _>(meta);
        let identifier = meta.identifier();
        match self.resources.get(&identifier) {
            Some(store) => Ok(store.clone()),
            None => {
                let resource = initialize(&meta, self)?;
                let store = Arc::new(unsafe { Store::new(meta, 1) });
                unsafe { store.set(0, resource) };
                self.resources.insert(identifier, store.clone());
                Ok(store)
            }
        }
    }

    fn get_segment_index(&self, types: &HashSet<TypeId>) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| segment.component_types() == types)
    }
}

impl Drop for World {
    fn drop(&mut self) {
        for (_, store) in &self.resources {
            unsafe { store.free(1, 1) };
        }
    }
}
