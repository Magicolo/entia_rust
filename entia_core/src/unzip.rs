use crate::recurse;
use std::mem::MaybeUninit;

pub trait Unzip {
    type Target;
    fn unzip(self) -> Self::Target;
}

macro_rules! tuple {
    () => {};
    ($p:ident, $t:ident) => {};
    ($($p:ident, $t:ident),*) => {
        impl<$($t,)* const N: usize> Unzip for [($($t,)*); N] {
            type Target = ($([$t; N],)*);

            #[inline]
            fn unzip(self) -> Self::Target {
                $(let mut $p = MaybeUninit::<[$t; N]>::uninit();)*
                {
                    $(let $p = $p.as_mut_ptr() as *mut $t;)*
                    let mut index = 0;
                    for ($($t,)*) in self {
                        unsafe { $($p.add(index).write($t);)* }
                        index += 1;
                    }
                }
                unsafe { ($($p.assume_init(),)*) }
            }
        }
    };
}

recurse!(tuple);
