use std::cell::UnsafeCell;

pub struct Wrap<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Wrap<T> {}

impl<T> Wrap<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    #[inline]
    pub unsafe fn get(&self) -> &mut T {
        &mut *self.0.get()
    }
}
