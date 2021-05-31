use crate::depend::Depend;
use crate::world::*;
use crate::{depend::Dependency, inject::*};
use std::ops::Deref;
use std::ops::DerefMut;

pub struct Local<'a, T: Default>(pub(crate) &'a mut T);
pub struct State<T>(T);

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
    type Input = Option<T>;
    type State = State<T>;

    fn initialize(input: Self::Input, _: &mut World) -> Option<Self::State> {
        Some(State(input.unwrap_or_default()))
    }
}

impl<'a, T: Default + 'static> Get<'a> for State<T> {
    type Item = Local<'a, T>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Local(&mut self.0)
    }
}

impl<T> Depend for State<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        Vec::new()
    }
}
