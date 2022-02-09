use crate::{
    any::{self, Any},
    generator::{
        constant::{constant, Constant},
        size::Size,
        FullGenerator, Generator, IntoGenerator, State,
    },
};
use std::{
    convert::TryInto,
    mem::size_of,
    ops::{self, Bound, Deref},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Full<T> {
    Generate,
    Shrink(Range<T>),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Range<T> {
    start: T,
    end: T,
    shrink: bool,
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
    type Generator = Any<(Constant<bool>, Constant<bool>), any::p14::One<Constant<bool>>>;
    fn generator() -> Self::Generator {
        Any::from((false.into(), true.into()))
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

        // impl Generator for $r {
        //     type Item = <Size<Range<$t>> as Generator>::Item;
        //     type State = <Size<Range<$t>> as Generator>::State;

        //     fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
        //         Range::<$t>::new(self.clone())
        //             .unwrap()
        //             .size()
        //             .generate(state)
        //     }

        //     fn shrink(&self, state: &mut Self::State) -> Option<Self> {
        //         Range::<$t>::new(self.clone())
        //             .unwrap()
        //             .size()
        //             .shrink(state)?
        //             .0
        //     }
        // }
    };
}

macro_rules! ranges {
    ($t:ident) => {
        impl FullGenerator for $t {
            type Item = $t;
            type Generator = Size<Full<$t>>;
            fn generator() -> Self::Generator {
                Full::Generate.size()
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
                if self.shrink {
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
                        shrink: self.shrink,
                    };
                    shrink
                } else {
                    self.clone()
                }
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
                Some(Self {
                    start,
                    end,
                    shrink: true,
                })
            }
        }

        #[inline]
        pub(super) fn to(&self) -> Range<u32> {
            Range {
                start: self.start as u32,
                end: self.end as u32,
                shrink: self.shrink,
            }
        }
    }

    impl Full<char> {
        fn special() -> impl Generator<Item = char> {
            const SPECIAL: [Constant<char>; 3] =
                constant!['\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER];
            Any::from(&SPECIAL).map(Option::unwrap)
        }
    }

    impl Generator for Range<char> {
        type Item = char;
        type State = char;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            let item = self.to().generate(state).0.try_into().unwrap();
            (item, item)
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self> {
            if self.start == self.end {
                None
            } else if *state < 0 as char {
                Some(Range {
                    start: *state,
                    end: self.end,
                    shrink: false,
                })
            } else {
                Some(Range {
                    start: self.start,
                    end: *state,
                    shrink: false,
                })
            }
        }
    }

    impl Generator for Size<Range<char>> {
        type Item = char;
        type State = char;

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

        fn shrink(&self, state: &mut Self::State) -> Option<Self> {
            Some(self.deref().shrink(state)?.size())
        }
    }

    impl Generator for Full<char> {
        type Item = char;
        type State = char;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            fn range(start: char, end: char, state: &mut State) -> char {
                Range {
                    start,
                    end,
                    shrink: true,
                }
                .to()
                .generate(state)
                .0
                .try_into()
                .unwrap()
            }

            match self {
                Self::Generate => {
                    let item = match state.random.u8(..) {
                        0..=250 => range('\u{0000}', '\u{D7FF}', state),
                        251..=254 => range('\u{E000}', char::MAX, state),
                        255 => Full::<char>::special().generate(state).0,
                    };
                    (item, item)
                }
                Self::Shrink(range) => range.generate(state),
            }
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self> {
            Some(Self::Shrink(match self {
                Self::Generate if *state >= '\u{E000}' => Range {
                    start: '\u{E000}',
                    end: *state,
                    shrink: false,
                },
                Self::Generate => Range {
                    start: '\u{0000}',
                    end: *state,
                    shrink: false,
                },
                Self::Shrink(range) => range.shrink(state)?,
            }))
        }
    }

    impl Generator for Size<Full<char>> {
        type Item = char;
        type State = char;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
            fn range(start: char, end: char, size: f64, state: &mut State) -> char {
                Range {
                    start,
                    end,
                    shrink: true,
                }
                .to()
                .shrinked(size.powi(size_of::<char>() as i32))
                .generate(state)
                .0
                .try_into()
                .unwrap()
            }

            match self.deref() {
                Full::Generate => {
                    let item = match state.random.u8(..) {
                        0..=250 => range('\u{0000}', '\u{D7FF}', state.size, state),
                        251..=254 => range('\u{E000}', char::MAX, state.size, state),
                        255 => Full::<char>::special().generate(state).0,
                    };
                    (item, item)
                }
                Full::Shrink(range) => range.generate(state),
            }
        }

        fn shrink(&self, state: &mut Self::State) -> Option<Self> {
            Some(self.deref().shrink(state)?.size())
        }
    }

    ranges!(char);
}

mod number {
    use super::*;

    macro_rules! integer {
        ($t:ident) => {
            impl Full<$t> {
                fn special() -> impl Generator<Item = $t> {
                    const SPECIAL: [Constant<$t>; 3] = constant![0 as $t, $t::MIN, $t::MAX];
                    Any::from(&SPECIAL).map(Option::unwrap)
                }
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
                            shrink: true,
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
                        shrink: true,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = state.random.$t(self.start..=self.end);
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    // TODO: Shrinking never stops...
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
                            shrink: false,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: *state,
                            shrink: false,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = self.shrinked(state.size).generate(state).0;
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(self.deref().shrink(state)?.size())
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    match self {
                        Self::Generate => {
                            let item = match state.random.u8(..) {
                                0..=254 => Range::from(Self::Generate).generate(state).0,
                                255 => Full::<$t>::special().generate(state).0,
                            };
                            (item, item)
                        },
                        Self::Shrink(range) => range.generate(state)
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(Self::Shrink(match self {
                        Self::Generate => Range::from(Self::Generate).shrink(state)?,
                        Self::Shrink(range) => range.shrink(state)?
                    }))
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = match state.random.u8(..) {
                        0..=254 => Range::from(Full::<$t>::Generate).shrinked(state.size.powi(size_of::<$t>() as i32)).generate(state).0,
                        255 => Full::<$t>::special().generate(state).0,
                    };
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(self.deref().shrink(state)?.size())
                }
            }

            ranges!($t);
        };
        ($($ts:ident),*) => { $(integer!($ts);)* };
    }

    macro_rules! floating {
        ($t:ident, $e:expr) => {
            impl Full<$t> {
                fn special() -> impl Generator<Item = $t> {
                    const SPECIAL: [Constant<$t>; 8] = constant![0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN];
                    Any::from(&SPECIAL).map(Option::unwrap)
                }
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
                            shrink: true,
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
                        shrink: true,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let ratio = state.random.$t();
                    let range = self.end - self.start;
                    let item = (range * ratio + self.start).max(self.start).min(self.end);
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    if self.start == self.end {
                        None
                    } else if *state < 0 as $t {
                        Some(Range {
                            start: *state,
                            end: self.end,
                            shrink: false,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: *state,
                            shrink: false,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    let item = self.shrinked(state.size).generate(state).0;
                    (item, item)
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(self.deref().shrink(state)?.size())
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    const FULL: Range<$t> = Range { start: $t::MIN, end: $t::MAX, shrink: true };

                    match self {
                        Self::Generate => {
                            let item = match state.random.u8(..) {
                                0..=126 => FULL.generate(state).0,
                                127..=253 => FULL.map(|value| 1 as $t / value).generate(state).0,
                                254..=255 => Full::<$t>::special().generate(state).0,
                            };
                            (item, item)
                        },
                        Self::Shrink(range) => range.generate(state),
                    }
                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(Self::Shrink(match self {
                        Self::Generate => Range::from(Self::Generate).shrink(state)?,
                        Self::Shrink(range) => range.shrink(state)?,
                    }))
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type State = $t;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::State) {
                    fn range(size: f64) -> Range::<$t> {
                        Range { start: -1 as $t / $t::EPSILON, end: 1 as $t / $t::EPSILON, shrink: true }
                            .shrinked(size.powi(size_of::<$t>() as i32))
                    }

                    match self.deref() {
                        Full::Generate => {
                            let item = match state.random.u8(..) {
                                0..=126 => range(state.size).generate(state).0,
                                127..=253 => range(state.size).map(|value| 1 as $t / value).generate(state).0,
                                254..=255 => Full::<$t>::special().generate(state).0,
                            };
                            (item, item)
                        },
                        Full::Shrink(range) => range.generate(state),
                    }

                }

                fn shrink(&self, state: &mut Self::State) -> Option<Self> {
                    Some(self.deref().shrink(state)?.size())
                }
            }

            ranges!($t);
        };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, 3.45266984e-4);
    floating!(f64, 1.4901161193847656e-8);
}
