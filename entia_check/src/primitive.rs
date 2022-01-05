use crate::generator::{size::Size, FullGenerator, Generator, IntoGenerator, State};
use std::{
    convert::TryInto,
    marker::PhantomData,
    mem::size_of,
    ops::{self, Bound, Deref},
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Full<T>(PhantomData<T>);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Range<T> {
    start: T,
    end: T,
}

impl<T> Copy for Full<T> {}
impl<T> Clone for Full<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T> ops::RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

impl FullGenerator for bool {
    type Item = Self;
    type Generator = [bool; 2];
    fn generator() -> Self::Generator {
        [false, true]
    }
}

macro_rules! range {
    ($t:ty, $r:ty) => {
        impl IntoGenerator for $r {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }

        impl Generator for $r {
            type Item = <Size<Range<$t>> as Generator>::Item;
            type State = <Size<Range<$t>> as Generator>::State;
            type Shrink = <Size<Range<$t>> as Generator>::Shrink;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                Range::<$t>::new(self.clone())
                    .unwrap()
                    .size()
                    .generate(state)
            }

            fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                Range::<$t>::new(self.clone()).unwrap().size().shrink(state)
            }
        }
    };
}

macro_rules! ranges {
    ($t:ident) => {
        impl FullGenerator for $t {
            type Item = $t;
            type Generator = Size<Full<$t>>;
            fn generator() -> Self::Generator {
                Full(PhantomData).size()
            }
        }

        range!($t, ops::Range<$t>);
        range!($t, ops::RangeInclusive<$t>);
        range!($t, ops::RangeFrom<$t>);
        range!($t, ops::RangeTo<$t>);
        range!($t, ops::RangeToInclusive<$t>);
    };
}

macro_rules! shrinked {
    ($t:ident) => {
        impl Range<$t> {
            pub(super) fn shrinked(&self, size: f64) -> Self {
                let start = self.start as f64;
                let end = self.end as f64;
                let range = end - start;
                let start_ratio = (end.min(0.) - start.min(0.)).abs() / range;
                let end_ratio = (end.max(0.) - start.max(0.)).abs() / range;
                let range_shrink = range * (1. - size);
                let start_shrink = start + range_shrink * start_ratio;
                let end_shrink = end - range_shrink * end_ratio;
                let shrink = Self {
                    start: (start_shrink.min(end_shrink) as $t)
                        .min(self.end)
                        .max(self.start),
                    end: (end_shrink.max(start_shrink) as $t)
                        .max(self.start)
                        .min(self.end),
                };
                shrink
            }
        }
    };
}

mod character {
    use super::*;

    impl Range<char> {
        // TODO: Return a 'Result' instead of 'Option'.
        pub fn new(range: impl ops::RangeBounds<char>) -> Option<Self> {
            let start = match range.start_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32).checked_add(1)?.try_into().ok()?,
                Bound::Unbounded => '\u{0000}',
            };
            let end = match range.end_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32).checked_sub(1)?.try_into().ok()?,
                Bound::Unbounded if start <= '\u{D7FF}' => '\u{D7FF}',
                Bound::Unbounded if start >= '\u{E000}' => char::MAX,
                Bound::Unbounded => return None,
            };
            if end < start {
                None
            } else {
                Some(Self { start, end })
            }
        }

        #[inline]
        pub(super) fn to(&self) -> Range<u32> {
            Range {
                start: self.start as u32,
                end: self.end as u32,
            }
        }
    }

    impl Full<char> {
        const SPECIAL: [char; 3] = ['\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER];
    }

    impl Generator for Range<char> {
        type Item = char;
        type State = char;
        type Shrink = Self;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let item = self.to().generate(state).0.try_into().unwrap();
            (item, item)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            if self.start == self.end {
                None
            } else if *state < 0 as char {
                Some(Range {
                    start: *state,
                    end: self.end,
                })
            } else {
                Some(Range {
                    start: self.start,
                    end: *state,
                })
            }
        }
    }

    impl Generator for Size<Range<char>> {
        type Item = char;
        type State = char;
        type Shrink = Range<char>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let item = self
                .to()
                .shrinked(state.size)
                .generate(state)
                .0
                .try_into()
                .unwrap();
            (item, item)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            self.deref().shrink(state)
        }
    }

    impl Generator for Full<char> {
        type Item = char;
        type State = char;
        type Shrink = Range<char>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            fn range(start: char, end: char, state: &mut State) -> char {
                Range { start, end }
                    .to()
                    .generate(state)
                    .0
                    .try_into()
                    .unwrap()
            }

            let item = match state.random.u8(..) {
                0..=250 => range('\u{0000}', '\u{D7FF}', state),
                251..=254 => range('\u{E000}', char::MAX, state),
                255 => (&Full::<char>::SPECIAL).generate(state).0,
            };
            (item, item)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            if *state >= '\u{E000}' {
                Some(Range {
                    start: '\u{E000}',
                    end: *state,
                })
            } else {
                Some(Range {
                    start: '\u{0000}',
                    end: *state,
                })
            }
        }
    }

    impl Generator for Size<Full<char>> {
        type Item = char;
        type State = char;
        type Shrink = Range<char>;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            fn range(start: char, end: char, size: f64, state: &mut State) -> char {
                Range { start, end }
                    .to()
                    .shrinked(size.powi(size_of::<char>() as i32))
                    .generate(state)
                    .0
                    .try_into()
                    .unwrap()
            }

            let item = match state.random.u8(..) {
                0..=250 => range('\u{0000}', '\u{D7FF}', state.size, state),
                251..=254 => range('\u{E000}', char::MAX, state.size, state),
                255 => (&Full::<char>::SPECIAL).generate(state).0,
            };
            (item, item)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
            self.deref().shrink(state)
        }
    }

    ranges!(char);
}

mod number {
    use super::*;

    macro_rules! integer {
        ($t:ident) => {
            impl Full<$t> {
                const SPECIAL: [$t; 3] = [0 as $t, $t::MIN, $t::MAX];
            }

            impl Range<$t> {
                // TODO: Return a 'Result' instead of 'Option'.
                pub fn new(range: impl ops::RangeBounds<$t>) -> Option<Self> {
                    let start = match range.start_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_add(1 as $t)?,
                        Bound::Unbounded => $t::MIN,
                    };
                    let end = match range.end_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_sub(1 as $t)?,
                        Bound::Unbounded => $t::MAX,
                    };
                    if end < start {
                        None
                    } else {
                        Some(Self {
                            start,
                            end,
                        })
                    }
                }
            }
            shrinked!($t);

            impl From<Full<$t>> for Range<$t> {
                #[inline]
                fn from(_: Full<$t>) -> Self {
                    Self {
                        start: $t::MIN,
                        end: $t::MAX,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type State = $t;
                type Shrink = Self;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = state.random.$t(self.start..=self.end);
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    if self.start == self.end {
                        None
                    } else if *state < $t::default() {
                        /*
                        TODO:
                        struct Shrink {
                            range: Range<$t>,
                            current: $t,
                        }

                        generate(&mut self) -> self.current

                        { 0..10, 10 }
                        { 0..5, x }, { 0..8, x }, { 0..9, 9 }
                        { 0..5, x }, { 0..7, x }, { 0..8, x }

                        { -10..10, 6 }, { -10..10, 8 }, { -10..10, 9 }
                        { 0..6, x }, { 0..8, x }, {  }
                         */
                        // TODO:
                        // - Take the middle point between 'last' and 'end.min(0)' and move gradually back towards 'last'.
                        // - This strategy shrinks in a greedy way and reverts...
                        Some(Range {
                            start: *state,
                            end: self.end,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: *state,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = self.shrinked(state.size).generate(state).0;
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    self.deref().shrink(state)
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = match state.random.u8(..) {
                        0..=254 => Range::from(*self).generate(state).0,
                        255 => (&Full::<$t>::SPECIAL).generate(state).0,
                    };
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    Range::from(*self).shrink(state)
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = match state.random.u8(..) {
                        0..=254 => Range::from(*self.deref()).shrinked(state.size.powi(size_of::<$t>() as i32)).generate(state).0,
                        255 => (&Full::<$t>::SPECIAL).generate(state).0,
                    };
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    self.deref().shrink(state)
                }
            }

            ranges!($t);
        };
        ($($ts:ident),*) => { $(integer!($ts);)* };
    }

    macro_rules! floating {
        ($t:ident, $e:expr) => {
            impl Full<$t> {
                const SPECIAL: [$t; 8] = [0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN];
            }

            impl Range<$t> {
                // TODO: Return a 'Result' instead of 'Option'.
                pub fn new(range: impl ops::RangeBounds<$t>) -> Option<Self> {
                    let start = match range.start_bound() {
                        Bound::Included(&bound) => (bound, false),
                        Bound::Excluded(&bound) => (bound, true),
                        Bound::Unbounded => ($t::MIN, false),
                    };
                    let end = match range.end_bound() {
                        Bound::Included(&bound) => (bound, false),
                        Bound::Excluded(&bound) => (bound, true),
                        Bound::Unbounded => ($t::MAX, false),
                    };

                    if end.0 < start.0 {
                        None
                    } else if (start.1 || end.1) && start.0 == end.0 {
                        None
                    } else {
                        let epsilon = (end.0 - start.0) * $e;
                        let start = if start.1 { start.0 + epsilon } else { start.0 };
                        let end = if end.1 { end.0 - epsilon } else { end.0 };
                        Some(Self {
                            start: start.min(end),
                            end: end.max(start),
                        })
                    }
                }
            }
            shrinked!($t);

            impl From<Full<$t>> for Range<$t> {
                #[inline]
                fn from(_: Full<$t>) -> Self {
                    Self {
                        start: $t::MIN,
                        end: $t::MAX,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type State = $t;
                type Shrink = Self;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let ratio = state.random.$t();
                    let range = self.end - self.start;
                    let item = (range * ratio + self.start).max(self.start).min(self.end);
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    if self.start == self.end {
                        None
                    } else if *state < 0 as $t {
                        Some(Range {
                            start: *state,
                            end: self.end,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: *state,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = self.shrinked(state.size).generate(state).0;
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    self.deref().shrink(state)
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    const FULL: Range<$t> = Range { start: $t::MIN, end: $t::MAX };
                    let item = match state.random.u8(..) {
                        0..=126 => FULL.generate(state).0,
                        127..=253 => FULL.map(|value| 1 as $t / value).generate(state).0,
                        254..=255 => (&Full::<$t>::SPECIAL).generate(state).0,
                    };
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    Range::from(*self).shrink(state)
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type State = $t;
                type Shrink = Range<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    fn range(size: f64) -> Range::<$t> {
                        Range { start: -1 as $t / $t::EPSILON, end: 1 as $t / $t::EPSILON }
                            .shrinked(size.powi(size_of::<$t>() as i32))
                    }

                    let item = match state.random.u8(..) {
                        0..=126 => range(state.size).generate(state).0,
                        127..=253 => range(state.size).map(|value| 1 as $t / value).generate(state).0,
                        254..=255 => (&Full::<$t>::SPECIAL).generate(state).0,
                    };
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self::Shrink> {
                    self.deref().shrink(state)
                }
            }

            ranges!($t);
        };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, 3.45266984e-4);
    floating!(f64, 1.4901161193847656e-8);
}
