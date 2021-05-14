pub trait Append<T> {
    type Target;
    fn append(self, value: T) -> Self::Target;
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
    };
}

crate::recurse_32!(append);
