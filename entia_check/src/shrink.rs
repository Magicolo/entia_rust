use crate::{map::Map, wrap::Wrap};

pub trait Shrink: Clone {
    type Item;
    fn generate(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;

    fn wrap<T, B: FnMut() -> T + Clone, A: FnMut(T) + Clone>(
        self,
        before: B,
        after: A,
    ) -> Wrap<Self, T, B, A>
    where
        Self: Sized,
    {
        Wrap::shrink(self, before, after)
    }

    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
    {
        Map::shrink(self, map)
    }
}
