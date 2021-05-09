pub trait Call<I, O> {
    fn call(&mut self, input: I) -> O;
}

macro_rules! call {
    ($($p:ident, $t:ident),*) => {
        impl<$($t,)* O, F: FnMut($($t,)*) -> O> Call<($($t,)*), O> for F {
            #[inline]
            fn call(&mut self, ($($p,)*): ($($t,)*)) -> O {
                self($($p,)*)
            }
        }
    };
}

crate::recurse!(
    call, input0, I0, input1, I1, input2, I2, input3, I3, input4, I4, input5, I5, input6, I6,
    input7, I7, input8, I8, input9, I9, input10, I10, input11, I11, input12, I12, input13, I13,
    input14, I14, input15, I15
);
