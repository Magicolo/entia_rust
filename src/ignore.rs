use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    query::item::{At, Item, ItemContext},
    world::World,
};

pub struct Ignore<I>(I);
pub struct State<S>(S);

impl<I> Ignore<I> {
    /// SAFETY: Since 'Ignore' removes the dependency checks that ensure Rust's invariants, the user must maintain them through some
    /// other means. This should not be used lightly since dependency logic can be quite tricky to get right.
    #[inline]
    pub unsafe fn get(self) -> I {
        self.0
    }
}

unsafe impl<I: Inject + 'static> Inject for Ignore<I> {
    type Input = I::Input;
    type State = State<I::State>;

    fn initialize(input: Self::Input, context: InjectContext) -> Option<Self::State> {
        Some(State(I::initialize(input, context)?))
    }
}

impl<'a, S: Get<'a>> Get<'a> for State<S> {
    type Item = Ignore<S::Item>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Ignore(self.0.get(world))
    }
}

unsafe impl<I: Item + 'static> Item for Ignore<I> {
    type State = State<I::State>;

    fn initialize(context: ItemContext) -> Option<Self::State> {
        Some(State(I::initialize(context)?))
    }
}

impl<'a, S: At<'a>> At<'a> for State<S> {
    type Item = Ignore<S::Item>;

    #[inline]
    fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
        Ignore(self.0.at(index, world))
    }
}

unsafe impl<S> Depend for State<S> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}
