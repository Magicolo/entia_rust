pub trait Call<I, O> {
    fn call(&self, input: I) -> O;
}

macro_rules! call {
    ($($p:ident, $t:ident),+) => {
        impl<$($t),+, O, F: Fn($($t),+) -> O> Call<($($t),+,), O> for F {
            #[inline]
            fn call(&self, ($($p),+,): ($($t),+,)) -> O {
                self($($p),+)
            }
        }
    };
}

crate::recurse!(
    call, input1, I1, input2, I2, input3, I3, input4, I4, input5, I5, input6, I6, input7, I7,
    input8, I8, input9, I9, input10, I10, input11, I11
);
