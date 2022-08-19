use crate::tuples;

pub trait Empty: Sized {
    const EMPTY: Self;
}

impl<T: Empty, const N: usize> Empty for [T; N] {
    const EMPTY: Self = [T::EMPTY; N];
}

impl<T: 'static> Empty for &[T] {
    const EMPTY: Self = &[];
}

impl<T> Empty for Option<T> {
    const EMPTY: Self = None;
}

impl<T, E: Empty> Empty for Result<T, E> {
    const EMPTY: Self = Err(E::EMPTY);
}

impl<T> Empty for Vec<T> {
    const EMPTY: Self = Vec::new();
}

macro_rules! unit {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Empty,)*> Empty for ($($t,)*) {
            const EMPTY: Self = ($(<$t as Empty>::EMPTY,)*);
        }
    };
}

tuples!(unit);
