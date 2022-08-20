use crate::tuples_with;

pub trait IntoArray<T, const N: usize> {
    fn array(self) -> [T; N];
}

macro_rules! tuple {
    ($n:ident, $c:tt $(, $p:ident, $t:ident, $i:tt)*) => {
        impl<T, $($t: Into<T>,)*> IntoArray<T, $c> for ($($t,)*) {
            #[inline]
            fn array(self) -> [T; $c] {
                [$(self.$i.into(),)*]
            }
        }
    };
}

tuples_with!(tuple);