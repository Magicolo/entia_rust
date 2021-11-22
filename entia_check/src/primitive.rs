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
            ['\0', char::MAX, char::REPLACEMENT_CHARACTER].generate(state)
        } else {
            let (start, end) = shrink('\0' as u32 as f64, char::MAX as u32 as f64, state.size);
            (char::from_u32(start as u32).unwrap()..=char::from_u32(end as u32).unwrap())
                .generate(state)
        }
    }
}

impl Generator for Size<Range<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = shrink(
            self.0.start as u32 as f64,
            self.0.end as u32 as f64,
            state.size,
        );
        (char::from_u32(start as u32).unwrap()..char::from_u32(end as u32).unwrap()).generate(state)
    }
}

impl Generator for Size<RangeInclusive<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = shrink(
            *self.0.start() as u32 as f64,
            *self.0.end() as u32 as f64,
            state.size,
        );
        (char::from_u32(start as u32).unwrap()..=char::from_u32(end as u32).unwrap())
            .generate(state)
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
        impl Generator for Full<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                state.random.$t(..)
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
                    [0, $t::MIN, $t::MAX].generate(state)
                } else {
                    let (start, end) = shrink($t::MIN as f64, $t::MAX as f64, state.size);
                    (start as $t..end as $t).generate(state)
                }
            }
        }

        impl Generator for Size<Range<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = shrink(self.0.start as f64, self.0.end as f64, state.size);
                (start as $t..end as $t).generate(state)
            }
        }

        impl Generator for Size<RangeInclusive<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = shrink(*self.0.start() as f64, *self.0.end() as f64, state.size);
                (start as $t..=end as $t).generate(state)
            }
        }

        full!($t);
        range!($t, $t::MIN, $t::MAX);
    };
    ($($ts:ident),*) => { $(integer!($ts);)* };
}

macro_rules! floating {
    ($t:ident) => {
        impl Generator for Full<$t> {
            type Item = $t;

            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                if state.random.bool() {
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
                state.random.$t() * (self.end - self.start) + self.start
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
                    [0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN].generate(state)
                } else {
                    let (start, end) = shrink($t::MIN as f64, $t::MAX as f64, state.size);
                    (start as $t..end as $t).generate(state)
                }
            }
        }

        impl Generator for Size<Range<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = shrink(self.0.start as f64, self.0.end as f64, state.size);
                (start as $t..end as $t).generate(state)
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

fn shrink(start: f64, end: f64, ratio: f64) -> (f64, f64) {
    let range = end - start;
    let start_ratio = (end.min(0.) - start.min(0.)).abs() / range;
    let end_ratio = (end.max(0.) - start.max(0.)).abs() / range;
    let range_shrink = range * ratio;
    let start_shrink = start + range_shrink * start_ratio;
    let end_shrink = end - range_shrink * end_ratio;
    (start_shrink, end_shrink)
}
