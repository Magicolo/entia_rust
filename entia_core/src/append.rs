use crate::tuples;

pub trait Append<T> {
    type Target: Chop<T, Rest = Self>;
    fn append(self, value: T) -> Self::Target;
}

pub trait Chop<T> {
    type Rest;
    fn chop(self) -> (T, Self::Rest);
}

macro_rules! append {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$($ts,)* $t> Append<$t> for ($($ts,)*) {
            type Target = ($($ts,)* $t,);

            #[inline]
            fn append(self, $p: $t) -> Self::Target {
                let ($($ps,)*) = self;
                ($($ps,)* $p,)
            }
        }

        impl<$($ts,)* $t> Chop<$t> for ($($ts,)* $t,) {
            type Rest = ($($ts,)*);

            #[inline]
            fn chop(self) -> ($t, Self::Rest) {
                let ($($ps,)* $p,) = self;
                ($p, ($($ps,)*))
            }
        }
    };
}

tuples!(append);
