pub trait Prepend<T> {
    type Target;
    fn prepend(self, value: T) -> Self::Target;
}

macro_rules! prepend {
    () => {};
    ($p:ident, $t:ident $(,$ps:ident, $ts:ident)*) => {
        impl<$t, $($ts,)*> Prepend<($($ts,)*)> for $t {
            type Target = ($($ts,)* $t,);

            #[inline]
            fn prepend(self, ($($ps,)*): ($($ts,)*)) -> Self::Target {
                ($($ps,)* self,)
            }
        }
    };
}

entia_macro::recurse_64!(prepend);
