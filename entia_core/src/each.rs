use crate::recurse;

pub trait EachRef<'a> {
    type Target;
    fn each_ref(&'a self) -> Self::Target;
}

pub trait EachMut<'a> {
    type Target;
    fn each_mut(&'a mut self) -> Self::Target;
}

macro_rules! tuple {
    () => {};
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: 'a,)*> EachRef<'a> for ($($t,)*) {
            type Target = ($(&'a $t,)*);

            #[inline]
            fn each_ref(&'a self) -> Self::Target {
                let ($($p,)*) = self;
                ($($p,)*)
            }
        }

        impl<'a, $($t: 'a,)*> EachMut<'a> for ($($t,)*) {
            type Target = ($(&'a mut $t,)*);

            #[inline]
            fn each_mut(&'a mut self) -> Self::Target {
                let ($($p,)*) = self;
                ($($p,)*)
            }
        }
    };
}

recurse!(tuple);
