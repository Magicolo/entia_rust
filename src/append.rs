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

crate::recurse!(
    append, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10, T10, p11,
    T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18, p19, T19, p20, T20,
    p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27, T27, p28, T28, p29, T29, p30,
    T30, p31, T31, p32, T32
);
