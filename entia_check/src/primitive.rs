use crate::generator::{Generator, IntoGenerator, Size, State};
use std::{
    marker::PhantomData,
    ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Full<T>(PhantomData<T>);

impl<T> Full<T> {
    #[inline]
    pub const fn new() -> Self {
        Full(PhantomData)
    }
}

impl IntoGenerator for bool {
    type Item = Self;
    type Generator = Full<bool>;
    #[inline]
    fn generator() -> Self::Generator {
        Full::new()
    }
}

impl Generator for Full<bool> {
    type Item = bool;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        state.random.bool()
    }
}

impl Full<char> {
    pub const SPECIAL: [char; 3] = ['\0', char::MAX, char::REPLACEMENT_CHARACTER];
}

impl Generator for Full<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        char::from_u32(('\0' as u32..=char::MAX as u32).generate(state)).unwrap()
    }
}

impl Generator for Range<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        char::from_u32((self.start as u32..self.end as u32).generate(state)).unwrap()
    }
}

impl Generator for RangeInclusive<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        char::from_u32((*self.start() as u32..=*self.end() as u32).generate(state)).unwrap()
    }
}

impl Generator for Size<Full<char>> {
    type Item = char;
    fn generate(&mut self, state: &mut State) -> Self::Item {
        if state.random.u8(..) == 0 {
            Full::<char>::SPECIAL.clone().generate(state)
        } else {
            Size('\0'..=char::MAX).generate(state)
        }
    }
}

impl Generator for Size<Range<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (self.0.start as u32, self.0.end as u32);
        char::from_u32(Size(start..end).generate(state)).unwrap()
    }
}

impl Generator for Size<RangeInclusive<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (*self.0.start() as u32, *self.0.end() as u32);
        char::from_u32(Size(start..=end).generate(state)).unwrap()
    }
}

macro_rules! full {
    ($t:ident) => {
        impl IntoGenerator for $t {
            type Item = <Self::Generator as Generator>::Item;
            type Generator = Size<Full<$t>>;
            #[inline]
            fn generator() -> Self::Generator {
                Size(Full::new())
            }
        }
    };
    ($($ts:ident),*) => { $(full!($ts);)* };
}

macro_rules! range {
    ($t:ident, $min:expr, $max:expr) => {
        impl Generator for RangeFrom<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                (self.start..=$max).generate(state)
            }
        }

        impl Generator for RangeTo<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                ($min..self.end).generate(state)
            }
        }

        impl Generator for RangeToInclusive<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                ($min..=self.end).generate(state)
            }
        }

        impl Generator for Size<RangeFrom<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Size((self.0.start..=$max)).generate(state)
            }
        }

        impl Generator for Size<RangeTo<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Size(($min..self.0.end)).generate(state)
            }
        }

        impl Generator for Size<RangeToInclusive<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Size(($min..=self.0.end)).generate(state)
            }
        }
    };
}

macro_rules! integer {
    ($t:ident) => {
        impl Full<$t> {
            pub const SPECIAL: [$t; 3] = [0 as $t, $t::MIN, $t::MAX];
        }

        impl Generator for Full<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                if state.random.u8(..) == 0 {
                    Full::<$t>::SPECIAL.clone().generate(state)
                } else {
                    state.random.$t(..)
                }
            }
        }

        impl Generator for Range<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                state.random.$t(self.clone())
            }
        }

        impl Generator for RangeInclusive<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                state.random.$t(self.clone())
            }
        }

        impl Generator for Size<Full<$t>> {
            type Item = $t;
            fn generate(&mut self, state: &mut State) -> Self::Item {
                if state.random.u8(..) == 0 {
                    Full::<$t>::SPECIAL.clone().generate(state)
                } else {
                    Size($t::MIN..=$t::MAX).generate(state)
                }
            }
        }

        impl Generator for Size<Range<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (self.0.start, self.0.end);
                let shrink = shrink(start as f64, end as f64, state.size, Some(1.));
                (shrink.0 as $t..shrink.1 as $t).generate(state)
            }
        }

        impl Generator for Size<RangeInclusive<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (*self.0.start(), *self.0.end());
                let shrink = shrink(start as f64, end as f64, state.size, None);
                (shrink.0 as $t..=shrink.1 as $t).generate(state)
            }
        }

        full!($t);
        range!($t, $t::MIN, $t::MAX);
    };
    ($($ts:ident),*) => { $(integer!($ts);)* };
}

macro_rules! floating {
    ($t:ident) => {
        impl Full<$t> {
            pub const SPECIAL: [$t; 8] = [0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN];
        }

        impl Generator for Full<$t> {
            type Item = $t;

            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                if state.random.u8(..) == 0 {
                    Full::<$t>::SPECIAL.clone().generate(state)
                } else if state.random.bool() {
                    state.random.$t() * $t::MAX
                } else {
                    state.random.$t() * $t::MIN
                }
            }
        }

        impl Generator for Range<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (self.start, self.end);
                if end - start < $t::EPSILON { panic!("empty range: {}..{}", start, end); }
                state.random.$t() * (end - start) + start
            }
        }

        impl Generator for RangeFrom<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                (self.start..$t::MAX).generate(state)
            }
        }

        impl Generator for RangeTo<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                ($t::MIN..self.end).generate(state)
            }
        }

        impl Generator for Size<Full<$t>> {
            type Item = $t;
            fn generate(&mut self, state: &mut State) -> Self::Item {
                if state.random.u8(..) == 0 {
                    Full::<$t>::SPECIAL.clone().generate(state)
                } else {
                    Size($t::MIN..$t::MAX).generate(state)
                }
            }
        }

        impl Generator for Size<Range<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (self.0.start, self.0.end);
                if end - start < $t::EPSILON { panic!("empty range: {}..{}", start, end); }
                let shrink = shrink(start as f64, end as f64, state.size, Some($t::EPSILON as f64));
                (shrink.0 as $t..shrink.1 as $t).generate(state)
            }
        }

        impl Generator for Size<RangeFrom<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Size(self.0.start..$t::MAX).generate(state)
            }
        }

        impl Generator for Size<RangeTo<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                Size($t::MIN..self.0.end).generate(state)
            }
        }

        full!($t);
    };
    ($($ts:ident),*) => { $(floating!($ts);)* };
}

full!(char);
range!(char, '\0', char::MAX);
integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
floating!(f32, f64);

fn shrink(start: f64, end: f64, size: f64, minimum: Option<f64>) -> (f64, f64) {
    let range = end - start;
    let start_ratio = (end.min(0.) - start.min(0.)).abs() / range;
    let end_ratio = (end.max(0.) - start.max(0.)).abs() / range;
    let range_shrink = range * (1. - size);
    let start_shrink = start + range_shrink * start_ratio;
    let end_shrink = end - range_shrink * end_ratio;

    match minimum {
        Some(minimum) if end_shrink - start_shrink < minimum => {
            if start_ratio > end_ratio {
                ((start_shrink - minimum).max(start), end_shrink)
            } else {
                (start_shrink, (end_shrink + minimum).min(end))
            }
        }
        _ => (start_shrink, end_shrink),
    }
}
