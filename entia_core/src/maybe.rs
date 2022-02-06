use std::marker::PhantomData;

pub struct Wrap<T: ?Sized, M: ?Sized>(PhantomData<T>, PhantomData<M>);

pub trait Maybe<T> {
    fn maybe(self) -> Option<T>;
}

impl<T, M> Default for Wrap<T, M> {
    fn default() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<T, M> Maybe<T> for &Wrap<T, M> {
    fn maybe(self) -> Option<T> {
        None
    }
}
