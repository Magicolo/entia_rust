use crate::recurse;

pub trait Merge {
    type Output;
    fn merge(self, other: Self) -> Self::Output;
}

impl<T: Merge, E> Merge for Result<T, E> {
    type Output = Result<T::Output, E>;

    #[inline]
    fn merge(self, other: Self) -> Self::Output {
        Ok(Merge::merge(self?, other?))
    }
}

impl<T: Merge> Merge for Option<T> {
    type Output = Option<T::Output>;

    #[inline]
    fn merge(self, other: Self) -> Self::Output {
        Some(Merge::merge(self?, other?))
    }
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Merge,)*> Merge for ($($t,)*) {
            type Output = ($($t::Output,)*);
            #[inline]
            fn merge(self, _other: Self) -> Self::Output {
                let ($($t,)*) = self;
                let ($($p,)*) = _other;
                ($($t.merge($p),)*)
            }
        }
    };
}

recurse!(tuple);
