use crate::{
    error::{Error, Result},
    identify,
    meta::Meta,
};
use std::{cell::Cell, ptr::NonNull, slice::from_raw_parts_mut, sync::Arc};

pub struct Store {
    identifier: usize,
    meta: Arc<Meta>,
    data: Cell<NonNull<()>>,
}

// SAFETY: 'Sync' and 'Send' can be implemented for 'Store' because this crate ensures its proper usage. Other users
// of this type must fulfill the safety requirements of its unsafe methods.
unsafe impl Sync for Store {}
unsafe impl Send for Store {}

impl Store {
    /// SAFETY: Owner of the 'Store' is responsible to track its 'count' and 'capacity' and to call 'free' whenever it is dropped.
    pub(crate) unsafe fn new(meta: Arc<Meta>, capacity: usize) -> Self {
        let data = Cell::new((meta.allocate)(capacity));
        Self {
            identifier: identify(),
            meta,
            data,
        }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn meta(&self) -> &Arc<Meta> {
        &self.meta
    }

    /// In order to be consistent with the requirements of 'Meta::new', 'T' is required to be 'Send + Sync'.
    #[inline]
    pub fn data<T: Send + Sync + 'static>(&self) -> *mut T {
        debug_assert!(self.meta().is::<T>());
        self.data.get().as_ptr().cast()
    }

    #[inline]
    pub unsafe fn copy(source: (&Self, usize), target: (&Self, usize), count: usize) {
        debug_assert_eq!(source.0.meta().identifier(), target.0.meta().identifier());
        (source.0.meta().copy)(
            (source.0.data.get(), source.1),
            (target.0.data.get(), target.1),
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
            (source.0.data.get(), source.1),
            (target.0.data.get(), target.1),
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
            (source.0.data.get(), source.1),
            (target.0.data.get(), target.1),
            count,
        );
        Ok(())
    }

    #[inline]
    pub unsafe fn chunk(&self, index: usize, count: usize) -> Result<Self> {
        let store = Self::new(self.meta.clone(), count);
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
    pub unsafe fn get_all<T: Send + Sync + 'static>(&self, count: usize) -> &mut [T] {
        from_raw_parts_mut(self.data::<T>(), count)
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

    #[inline]
    pub unsafe fn replace<T: Send + Sync + 'static>(&self, index: usize, item: T) -> T {
        let pointer = self.data::<T>().add(index);
        let value = pointer.read();
        pointer.write(item);
        value
    }

    /// SAFETY: Both the 'source' and 'target' indices must be within the bounds of the store.
    /// The ranges 'source_index..source_index + count' and 'target_index..target_index + count' must not overlap.
    #[inline]
    pub unsafe fn squash(&self, source_index: usize, target_index: usize, count: usize) {
        let meta = self.meta();
        let pointer = self.data.get();
        (meta.drop)(pointer, target_index, count);
        (meta.copy)((pointer, source_index), (pointer, target_index), count);
    }

    #[inline]
    pub unsafe fn drop(&self, index: usize, count: usize) {
        (self.meta().drop)(self.data.get(), index, count);
    }

    #[inline]
    pub unsafe fn free(&self, count: usize, capacity: usize) {
        (self.meta().free)(self.data.get(), count, capacity);
        self.data.set(NonNull::dangling());
    }

    pub unsafe fn grow(&self, old_capacity: usize, new_capacity: usize) {
        debug_assert!(old_capacity < new_capacity);
        let meta = self.meta();
        let old_pointer = self.data.get();
        let new_pointer = (self.meta().allocate)(new_capacity);
        (meta.copy)((old_pointer, 0), (new_pointer, 0), old_capacity);
        (meta.free)(old_pointer, 0, old_capacity);
        self.data.set(new_pointer);
    }
}
