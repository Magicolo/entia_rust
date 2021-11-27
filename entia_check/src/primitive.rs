use crate::generator::{Generator, IntoGenerator, Size, State};
use std::{
    convert::TryInto,
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
    pub const SPECIAL: [char; 3] = ['\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER];
}

impl Generator for Full<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        match state.random.u8(..) {
            0 => Full::<char>::SPECIAL.clone().generate(state),
            1..=4 => ('\u{E000}'..=char::MAX).generate(state),
            _ => ('\u{0000}'..='\u{D7FF}').generate(state),
        }
    }
}

impl Generator for Range<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (self.start as u32, self.end as u32);
        (start..end).generate(state).try_into().unwrap()
    }
}

impl Generator for RangeInclusive<char> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (*self.start() as u32, *self.end() as u32);
        (start..=end).generate(state).try_into().unwrap()
    }
}

impl Generator for Size<Full<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        match state.random.u8(..) {
            0 => Full::<char>::SPECIAL.clone().generate(state),
            1..=4 => Size('\u{E000}'..=char::MAX).generate(state),
            _ => Size('\u{0000}'..='\u{D7FF}').generate(state),
        }
    }
}

impl Generator for Size<Range<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (self.0.start as u32, self.0.end as u32);
        Size(start..end).generate(state).try_into().unwrap()
    }
}

impl Generator for Size<RangeInclusive<char>> {
    type Item = char;
    #[inline]
    fn generate(&mut self, state: &mut State) -> Self::Item {
        let (start, end) = (*self.0.start() as u32, *self.0.end() as u32);
        Size(start..=end).generate(state).try_into().unwrap()
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

macro_rules! number {
    ($t:ident) => {
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
                match state.random.u8(..) {
                    0 => Full::<$t>::SPECIAL.clone().generate(state),
                    _ => ($t::MIN..=$t::MAX).generate(state),
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
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                match state.random.u8(..) {
                    0 => Full::<$t>::SPECIAL.clone().generate(state),
                    _ => Size($t::MIN..=$t::MAX).generate(state),
                }
            }
        }

        number!($t);
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
                match state.random.u8(..) {
                    0..=1 => Full::<$t>::SPECIAL.clone().generate(state),
                    2..=64 => (0 as $t..=$t::MAX).map(|value| 1. / value).generate(state),
                    65..=127 => ($t::MIN..=0 as $t).map(|value| 1. / value).generate(state),
                    128..=191 => (0 as $t..=$t::MAX).generate(state),
                    192..=255 => ($t::MIN..=0 as $t).generate(state),
                }
            }
        }

        impl Generator for Range<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (self.start, self.end);
                let range = end - start;
                if range < $t::EPSILON { panic!("empty range: {}..{}", start, end); }
                state.random.$t() * (range - $t::EPSILON) + start
            }
        }

        impl Generator for RangeInclusive<$t> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                let (start, end) = (*self.start(), *self.end());
                state.random.$t() * (end - start) + start
            }
        }

        impl Generator for Size<Full<$t>> {
            type Item = $t;
            #[inline]
            fn generate(&mut self, state: &mut State) -> Self::Item {
                match state.random.u8(..) {
                    0 | u8::MAX => Full::<$t>::SPECIAL.clone().generate(state),
                    1..=127 => Size(0 as $t..=$t::MAX).generate(state),
                    128..=254 => Size($t::MIN..=0 as $t).generate(state),
                }
            }
        }

        number!($t);
    };
    ($($ts:ident),*) => { $(floating!($ts);)* };
}

full!(char);
range!(char, '\u{0000}', char::MAX);
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
