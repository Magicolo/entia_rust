use crate::internal::*;
use crate::system::*;
use crate::*;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

pub struct State<T: Default>(Arc<Wrap<T>>);

impl<T: Default> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.get() }
    }
}

impl<T: Default> DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.get() }
    }
}

impl<T: Default> Inject for State<T> {
    type State = Arc<Wrap<T>>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(Arc::new(Wrap::new(T::default())))
    }

    fn update(_: &mut Self::State, _: &mut World) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State, _: &mut World) {}

    #[inline]
    fn inject(state: &Self::State, _: &World) -> Self {
        State(state.clone())
    }
}
