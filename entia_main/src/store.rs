use crate::{
    error::{Error, Result},
    meta::Meta,
};
use std::{any::TypeId, cell::Cell, slice::from_raw_parts_mut, sync::Arc};

pub struct Store(Arc<Meta>, Cell<*mut ()>);

// SAFETY: 'Sync' and 'Send' can be implemented for 'Store' because this crate ensures its proper usage. Other users
// of this type must fulfill the safety requirements of its unsafe methods.
unsafe impl Sync for Store {}
unsafe impl Send for Store {}

impl Store {
    /// SAFETY: Owner of the 'Store' is responsible to track its 'count' and 'capacity' and to call 'free' whenever it is dropped.
    pub(crate) unsafe fn new(meta: Arc<Meta>, capacity: usize) -> Self {
        let pointer = (meta.allocate)(capacity);
        Self(meta, pointer.into())
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        &self.0
    }

    /// In order to be consistent with the requirements of 'Meta::new', 'T' is required to be 'Send + Sync'.
    #[inline]
    pub fn data<T: Send + Sync + 'static>(&self) -> *mut T {
        debug_assert_eq!(TypeId::of::<T>(), self.meta().identifier());
        self.1.get().cast()
    }

    #[inline]
    pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
        debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
        (source.0.meta().copy)(
            (source.0 .1.get(), source.1),
            (target.0 .1.get(), target.1),
            count,
        );
    }

    /// SAFETY: The target must be dropped before calling this function.
    pub unsafe fn clone(source: (&Self, usize), target: (&Self, usize), count: usize) -> Result {
        debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
        let metas = (source.0.meta(), target.0.meta());
        let cloners = (
            metas.0.cloner.as_ref().ok_or(Error::MissingClone {
                name: metas.0.name(),
            }),
            metas.1.cloner.as_ref().ok_or(Error::MissingClone {
                name: metas.1.name(),
            }),
        );
        let cloner = cloners.0.or(cloners.1)?;
        (cloner.clone)(
            (source.0 .1.get(), source.1),
            (target.0 .1.get(), target.1),
            count,
        );
        Ok(())
    }

    /// SAFETY: The target must be dropped before calling this function.
    pub unsafe fn fill(source: (&Self, usize), target: (&Self, usize), count: usize) -> Result {
        debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
        let metas = (source.0.meta(), target.0.meta());
        let error = Error::MissingClone {
            name: metas.0.name(),
        };
        let cloner = metas
            .0
            .cloner
            .as_ref()
            .or(metas.1.cloner.as_ref())
            .ok_or(error)?;
        (cloner.fill)(
            (source.0 .1.get(), source.1),
            (target.0 .1.get(), target.1),
            count,
        );
        Ok(())
    }

    #[inline]
    pub unsafe fn chunk(&self, index: usize, count: usize) -> Result<Self> {
        let store = Self::new(self.0.clone(), count);
        match Self::clone((self, index), (&store, 0), count) {
            Ok(_) => Ok(store),
            Err(error) => {
                store.free(0, count);
                Err(error)
            }
        }
    }

    /// SAFETY: The 'index' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get<T: Send + Sync + 'static>(&self, index: usize) -> &mut T {
        &mut *self.data::<T>().add(index)
    }

    /// SAFETY: Both 'index' and 'count' must be within the bounds of the store.
    #[inline]
    pub unsafe fn get_all<T: Send + Sync + 'static>(&self, index: usize, count: usize) -> &mut [T] {
        from_raw_parts_mut(self.data::<T>().add(index), count)
    }

    #[inline]
    pub unsafe fn set<T: Send + Sync + 'static>(&self, index: usize, item: T) {
        self.data::<T>().add(index).write(item);
    }

    #[inline]
    pub unsafe fn set_all<T: Send + Sync + 'static>(&self, index: usize, items: &[T])
    where
        T: Copy,
    {
        let source = items.as_ptr().cast::<T>();
        let target = self.data::<T>().add(index);
        source.copy_to_nonoverlapping(target, items.len());
    }

    /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
    /// The ranges 'source_index..source_index + count' and 'target_index..target_index + count' must not overlap.
    #[inline]
    pub unsafe fn squash(&self, source_index: usize, target_index: usize, count: usize) {
        let meta = self.meta();
        let pointer = self.1.get();
        (meta.drop)(pointer, target_index, count);
        (meta.copy)((pointer, source_index), (pointer, target_index), count);
    }

    #[inline]
    pub unsafe fn drop(&self, index: usize, count: usize) {
        (self.meta().drop)(self.1.get(), index, count);
    }

    #[inline]
    pub unsafe fn free(&self, count: usize, capacity: usize) {
        (self.meta().free)(self.1.get(), count, capacity);
    }

    pub unsafe fn resize(&self, old_capacity: usize, new_capacity: usize) {
        let meta = self.meta();
        let old_pointer = self.1.get();
        let new_pointer = (self.meta().allocate)(new_capacity);
        (meta.copy)((old_pointer, 0), (new_pointer, 0), old_capacity);
        (meta.free)(old_pointer, 0, old_capacity);
        self.1.set(new_pointer);
    }
}