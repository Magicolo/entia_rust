use crate::generator::{size::Size, FullGenerator, Generator, IntoGenerator, State};
use std::{
    convert::TryInto,
    mem::size_of,
    ops::{self, Bound, DerefMut},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Full<T>(T);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Range<T> {
    start: T,
    end: T,
    last: T,
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
    #[inline]
    fn generator() -> Self::Generator {
        [false, true]
    }
}

macro_rules! range {
    ($t:ident) => {
        impl FullGenerator for $t {
            type Item = $t;
            type Generator = Size<Full<$t>>;
            #[inline]
            fn generator() -> Self::Generator {
                Full($t::default()).size()
            }
        }

        impl IntoGenerator for ops::Range<$t> {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }

        impl IntoGenerator for ops::RangeInclusive<$t> {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }

        impl IntoGenerator for ops::RangeFrom<$t> {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }

        impl IntoGenerator for ops::RangeTo<$t> {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }

        impl IntoGenerator for ops::RangeToInclusive<$t> {
            type Item = $t;
            type Generator = Size<Range<$t>>;
            fn generator(self) -> Self::Generator {
                Range::<$t>::new(self).unwrap().size()
            }
        }
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
                    last: self.last,
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
                Some(Self {
                    start,
                    end,
                    last: start,
                })
            }
        }

        #[inline]
        fn to(&self) -> Range<u32> {
            Range {
                start: self.start as u32,
                end: self.end as u32,
                last: self.last as u32,
            }
        }
    }

    impl Full<char> {
        const SPECIAL: [char; 3] = ['\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER];
    }

    impl Generator for Range<char> {
        type Item = char;
        type Shrink = Self;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self.last = self.to().generate(state).try_into().unwrap();
            self.last
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            if self.start == self.end {
                None
            } else if self.last < 0 as char {
                Some(Range {
                    start: self.last,
                    end: self.end,
                    last: self.last,
                })
            } else {
                Some(Range {
                    start: self.start,
                    end: self.last,
                    last: self.last,
                })
            }
        }
    }

    impl Generator for Size<Range<char>> {
        type Item = char;
        type Shrink = Range<char>;

        #[inline]
        fn generate(&mut self, state: &mut State) -> Self::Item {
            self.last = self
                .to()
                .shrinked(state.size)
                .generate(state)
                .try_into()
                .unwrap();
            self.last
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            self.deref_mut().shrink()
        }
    }

    impl Generator for Full<char> {
        type Item = char;
        type Shrink = Range<char>;

        fn generate(&mut self, state: &mut State) -> Self::Item {
            #[inline]
            fn range(start: char, end: char, last: char) -> Range<u32> {
                Range { start, end, last }.to()
            }

            self.0 = match state.random.u8(..) {
                0..=250 => range('\u{0000}', '\u{D7FF}', self.0)
                    .generate(state)
                    .try_into()
                    .unwrap(),
                251..=254 => range('\u{E000}', char::MAX, self.0)
                    .generate(state)
                    .try_into()
                    .unwrap(),
                255 => Full::<char>::SPECIAL.clone().generate(state),
            };
            self.0
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            if self.0 >= '\u{E000}' {
                Some(Range {
                    start: '\u{E000}',
                    end: self.0,
                    last: self.0,
                })
            } else {
                Some(Range {
                    start: '\u{0000}',
                    end: self.0,
                    last: self.0,
                })
            }
        }
    }

    impl Generator for Size<Full<char>> {
        type Item = char;
        type Shrink = Range<char>;

        fn generate(&mut self, state: &mut State) -> Self::Item {
            #[inline]
            fn range(start: char, end: char, last: char, size: f64) -> Range<u32> {
                Range { start, end, last }
                    .to()
                    .shrinked(size.powi(size_of::<char>() as i32))
            }

            self.0 = match state.random.u8(..) {
                0..=250 => range('\u{0000}', '\u{D7FF}', '\u{0000}', state.size)
                    .generate(state)
                    .try_into()
                    .unwrap(),
                251..=254 => range('\u{E000}', char::MAX, char::MAX, state.size)
                    .generate(state)
                    .try_into()
                    .unwrap(),
                255 => Full::<char>::SPECIAL.clone().generate(state),
            };
            self.0
        }

        fn shrink(&mut self) -> Option<Self::Shrink> {
            self.deref_mut().shrink()
        }
    }

    range!(char);
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
                            last: start,
                        })
                    }
                }
            }
            shrinked!($t);

            impl From<Full<$t>> for Range<$t> {
                #[inline]
                fn from(full: Full<$t>) -> Self {
                    Self {
                        start: $t::MIN,
                        end: $t::MAX,
                        last: full.0,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type Shrink = Self;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self.last = state.random.$t(self.start..=self.end);
                    self.last
                }

                fn shrink(&mut self) -> Option<Self::Shrink> {
                    if self.start == self.end {
                        None
                    } else if self.last < $t::default() {
                        Some(Range {
                            start: self.last,
                            end: self.end,
                            last: self.last,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: self.last,
                            last: self.last,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type Shrink = Range<$t>;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self.last = self.shrinked(state.size).generate(state);
                    self.last
                }

                fn shrink(&mut self) -> Option<Self::Shrink> {
                    self.deref_mut().shrink()
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type Shrink = Range<$t>;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self.0 = match state.random.u8(..) {
                        0..=254 => Range::from(*self).generate(state),
                        255 => Full::<$t>::SPECIAL.clone().generate(state),
                    };
                    self.0
                }

                #[inline]
                fn shrink(&mut self) -> Option<Self::Shrink> {
                    Range::from(*self).shrink()
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type Shrink = Range<$t>;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self.0 = match state.random.u8(..) {
                        0..=254 => Range::from(**self).shrinked(state.size.powi(size_of::<$t>() as i32)).generate(state),
                        255 => Full::<$t>::SPECIAL.clone().generate(state),
                    };
                    self.0
                }

                #[inline]
                fn shrink(&mut self) -> Option<Self::Shrink> {
                    self.deref_mut().shrink()
                }
            }

            range!($t);
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
                            last: start,
                        })
                    }
                }
            }
            shrinked!($t);

            impl From<Full<$t>> for Range<$t> {
                #[inline]
                fn from(full: Full<$t>) -> Self {
                    Self {
                        start: $t::MIN,
                        end: $t::MAX,
                        last: full.0,
                    }
                }
            }

            impl Generator for Range<$t> {
                type Item = $t;
                type Shrink = Self;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    let ratio = state.random.$t();
                    let range = self.end - self.start;
                    self.last = (range * ratio + self.start).max(self.start).min(self.end);
                    self.last
                }

                fn shrink(&mut self) -> Option<Self::Shrink> {
                    if self.start == self.end {
                        None
                    } else if self.last < 0 as $t {
                        Some(Range {
                            start: self.last,
                            end: self.end,
                            last: self.last,
                        })
                    } else {
                        Some(Range {
                            start: self.start,
                            end: self.last,
                            last: self.last,
                        })
                    }
                }
            }

            impl Generator for Size<Range<$t>> {
                type Item = $t;
                type Shrink = Range<$t>;

                #[inline]
                fn generate(&mut self, state: &mut State) -> Self::Item {
                    self.last = self.shrinked(state.size).generate(state);
                    self.last
                }

                fn shrink(&mut self) -> Option<Self::Shrink> {
                    self.deref_mut().shrink()
                }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type Shrink = Range<$t>;

                fn generate(&mut self, state: &mut State) -> Self::Item {
                    #[inline]
                    fn range(last: $t) -> Range::<$t> {
                        Range { start: $t::MIN, end: $t::MAX, last }
                    }

                    self.0 = match state.random.u8(..) {
                        0..=126 => range(self.0).generate(state),
                        127..=253 => range(self.0).map(|value| 1 as $t / value).generate(state),
                        254..=255 => Full::<$t>::SPECIAL.clone().generate(state),
                    };
                    self.0
                }

                #[inline]
                fn shrink(&mut self) -> Option<Self::Shrink> {
                    Range::from(*self).shrink()
                }
            }

            impl Generator for Size<Full<$t>> {
                type Item = $t;
                type Shrink = Range<$t>;

                fn generate(&mut self, state: &mut State) -> Self::Item {
                    #[inline]
                    fn range(last: $t, size: f64) -> Range::<$t> {
                        Range { start: -1 as $t / $t::EPSILON, end: 1 as $t / $t::EPSILON, last }.shrinked(size.powi(size_of::<$t>() as i32))
                    }

                    self.0 = match state.random.u8(..) {
                        0..=126 => range(self.0, state.size).generate(state),
                        127..=253 => range(self.0, state.size).map(|value| 1 as $t / value).generate(state),
                        254..=255 => Full::<$t>::SPECIAL.clone().generate(state),
                    };
                    self.0
                }

                #[inline]
                fn shrink(&mut self) -> Option<Self::Shrink> {
                    self.deref_mut().shrink()
                }
            }

            range!($t);
        };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, 3.45266984e-4);
    floating!(f64, 1.4901161193847656e-8);
}
