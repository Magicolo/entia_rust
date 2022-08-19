use crate::tuples;
use std::mem::MaybeUninit;

pub trait Unzip {
    type Target;
    fn unzip(self) -> Self::Target;
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t,)* const N: usize> Unzip for [($($t,)*); N] {
            type Target = ($([$t; N],)*);

            #[inline]
            fn unzip(self) -> Self::Target {
                $(let mut $p = MaybeUninit::<[$t; N]>::uninit();)*
                {
                    $(let $p = $p.as_mut_ptr() as *mut $t;)*
                    for (_i, ($($t,)*)) in self.into_iter().enumerate() {
                        $(unsafe { $p.add(_i).write($t); })*
                    }
                }
                ($(unsafe { $p.assume_init() },)*)
            }
        }
    };
}

tuples!(tuple);
