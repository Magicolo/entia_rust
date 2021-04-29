use std::cell::UnsafeCell;

pub struct Wrap<T>(pub(crate) UnsafeCell<T>);

unsafe impl<T> Sync for Wrap<T> {}
impl<T> Wrap<T> {
    pub fn new(value: T) -> Self {
        Wrap(UnsafeCell::new(value))
    }
}
