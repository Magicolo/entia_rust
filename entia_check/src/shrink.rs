use crate::{map::Map, wrap::Wrap};

pub trait Shrink: Clone {
    type Item;
    fn generate(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;

    fn wrap<T, B: Fn() -> T, A: Fn(T)>(self, before: B, after: A) -> Wrap<Self, T, B, A>
    where
        Wrap<Self, T, B, A>: Shrink,
    {
        Wrap::shrinker(self, before, after)
    }

    fn map<T, F: Fn(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Map<Self, T, F>: Shrink,
    {
        Map::shrink(self, map)
    }
}
