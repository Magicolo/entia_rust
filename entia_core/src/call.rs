use crate::tuples;

pub trait Call<I, O = ()> {
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

tuples!(call);
