use std::marker::PhantomData;

pub struct Wrap<T: ?Sized>(PhantomData<T>);

pub trait Maybe<T> {
    fn maybe(self) -> Option<T>;
}

impl<T> Default for Wrap<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> Maybe<T> for &Wrap<T> {
    fn maybe(self) -> Option<T> {
        None
    }
}
