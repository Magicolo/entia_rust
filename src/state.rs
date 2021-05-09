use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::cell::UnsafeCell;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct State<'a, T: Default>(pub(crate) &'a mut T);

impl<T: Default> AsRef<T> for State<'_, T> {
    fn as_ref(&self) -> &T {
        self.0
    }
}

impl<T: Default> AsMut<T> for State<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.0
    }
}

impl<T: Default> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: Default> DerefMut for State<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a, T: Default + 'a> Inject<'a> for State<'a, T> {
    type State = UnsafeCell<T>;

    fn initialize(_: &World) -> Option<Self::State> {
        Some(T::default().into())
    }

    fn inject(state: &Self::State) -> Self {
        State(unsafe { &mut *state.get() })
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}
