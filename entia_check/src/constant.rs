pub use crate::constant_expressions as constant;
use crate::{
    generator::{Generate, State},
    shrink::Shrink,
};

#[macro_export]
macro_rules! constant_expressions {
        ($($e:expr),*) => { [$($crate::generator::constant::Constant($e)),*] };
    }

#[derive(Clone, Debug, Default)]
pub struct Constant<T>(pub T);

impl<T> From<T> for Constant<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: Clone> Generate for Constant<T> {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
        (self.0.clone(), self.clone())
    }
}

impl<T: Clone> Shrink for Constant<T> {
    type Item = T;

    fn generate(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
