pub trait Indirect<'a, M> {}
pub trait Marker<T> {}

impl<'a, T, M: Marker<T>> Indirect<'a, T> for M {}
