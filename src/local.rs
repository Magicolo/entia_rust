use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct Local<'a, T: Default>(pub(crate) &'a mut T);
pub struct LocalState<T>(T);

impl<T: Default> AsRef<T> for Local<'_, T> {
    fn as_ref(&self) -> &T {
        self.0
    }
}

impl<T: Default> AsMut<T> for Local<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.0
    }
}

impl<T: Default> Deref for Local<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: Default> DerefMut for Local<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T: Default + 'static> Inject for Local<'_, T> {
    type State = LocalState<T>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(LocalState(T::default()))
    }

    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}

impl<'a, T: Default + 'a> Get<'a> for LocalState<T> {
    type Item = Local<'a, T>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Local(&mut self.0)
    }
}
