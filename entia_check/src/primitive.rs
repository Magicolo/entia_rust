use crate::{
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::Shrink,
    size::Size,
};
use entia_core::Change;
use std::{
    convert::TryInto,
    marker::PhantomData,
    mem::size_of,
    ops::{self, Bound, Deref},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Full<T: ?Sized>(PhantomData<T>);

#[derive(Copy, Clone, Debug, Default)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Overflow,
    Empty,
    Invalid,
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    None,
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct Shrinker<T> {
    pub range: Range<T>,
    pub item: T,
    pub direction: Direction,
}

impl<T: ?Sized> Full<T> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Range<T> {
    #[inline]
    pub fn map<U, F: FnMut(T) -> U>(self, mut map: F) -> Range<U> {
        Range {
            start: map(self.start),
            end: map(self.end),
        }
    }
}

impl<T> Shrinker<T> {
    #[inline]
    pub const fn new(range: Range<T>, item: T) -> Self {
        Self {
            range,
            item,
            direction: Direction::None,
        }
    }

    #[inline]
    pub fn map<U, F: FnMut(T) -> U>(self, mut map: F) -> Shrinker<U> {
        let item = map(self.item);
        Shrinker {
            range: self.range.map(map),
            item,
            direction: self.direction,
        }
    }
}

impl From<Range<char>> for Range<u32> {
    #[inline]
    fn from(value: Range<char>) -> Self {
        value.map(|value| value as u32)
    }
}

impl From<Shrinker<char>> for Shrinker<u32> {
    #[inline]
    fn from(value: Shrinker<char>) -> Self {
        value.map(|value| value as u32)
    }
}

impl TryFrom<Range<u32>> for Range<char> {
    type Error = <char as TryFrom<u32>>::Error;

    #[inline]
    fn try_from(value: Range<u32>) -> Result<Self, Self::Error> {
        Ok(Self {
            start: value.start.try_into()?,
            end: value.end.try_into()?,
        })
    }
}

impl TryFrom<Shrinker<u32>> for Shrinker<char> {
    type Error = <char as TryFrom<u32>>::Error;

    #[inline]
    fn try_from(value: Shrinker<u32>) -> Result<Self, Self::Error> {
        Ok(Self {
            range: value.range.try_into()?,
            item: value.item.try_into()?,
            direction: value.direction,
        })
    }
}

impl<T> ops::RangeBounds<T> for Range<T> {
    #[inline]
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    #[inline]
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

macro_rules! constant {
    ($t:ty) => {
        impl Generate for $t {
            type Item = Self;
            type Shrink = Self;

            fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
                (*self, *self)
            }
        }

        impl Shrink for $t {
            type Item = Self;

            fn generate(&self) -> Self::Item {
                *self
            }

            fn shrink(&mut self) -> Option<Self> {
                None
            }
        }
    };
}

macro_rules! range {
    ($t:ty, $r:ty) => {
        impl TryFrom<$r> for Range<$t> {
            type Error = Error;
            #[inline]
            fn try_from(range: $r) -> Result<Self, Self::Error> {
                Range::<$t>::new(range)
            }
        }

        impl TryFrom<$r> for Size<Range<$t>> {
            type Error = Error;
            #[inline]
            fn try_from(range: $r) -> Result<Self, Self::Error> {
                Ok(Range::<$t>::try_from(range)?.size())
            }
        }

        impl IntoGenerate for $r {
            type Item = $t;
            type Generate = Size<Range<$t>>;
            fn generator(self) -> Self::Generate {
                self.try_into().unwrap()
            }
        }

        impl Generate for $r {
            type Item = <Size<Range<$t>> as Generate>::Item;
            type Shrink = <Size<Range<$t>> as Generate>::Shrink;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                self.clone().generator().generate(state)
            }
        }
    };
}

macro_rules! ranges {
    ($t:ident) => {
        impl FullGenerate for $t {
            type Item = $t;
            type Generate = Size<Full<$t>>;
            fn generator() -> Self::Generate {
                Full(PhantomData).size()
            }
        }

        impl TryFrom<ops::RangeFull> for Range<$t> {
            type Error = Error;
            #[inline]
            fn try_from(range: ops::RangeFull) -> Result<Self, Self::Error> {
                Range::<$t>::new(range)
            }
        }

        impl TryFrom<ops::RangeFull> for Size<Range<$t>> {
            type Error = Error;
            #[inline]
            fn try_from(range: ops::RangeFull) -> Result<Self, Self::Error> {
                Ok(Range::<$t>::try_from(range)?.size())
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

macro_rules! shrink {
    ($s:expr, $t:ident) => {{
        let range = &mut $s.range;
        match $s.direction {
            Direction::None if range.start < range.end && $s.item > 0 as $t => {
                let delta = Range::<$t>::delta(range.start, $s.item, 2 as $t).max(1 as $t);
                let item = ($s.item - delta).max(0 as $t).max(range.start);
                range.end = $s.item;
                $s.item = item;
                $s.direction = Direction::Right;
                Some(Self::new(range.clone(), $s.item))
            }
            Direction::None if range.start < range.end && $s.item < 0 as $t => {
                let delta = Range::<$t>::delta(range.end, $s.item, 2 as $t).max(1 as $t);
                let item = ($s.item + delta).min(0 as $t).min(range.end);
                range.start = $s.item;
                $s.item = item;
                $s.direction = Direction::Left;
                Some(Self::new(range.clone(), $s.item))
            }
            Direction::Left if $s.item > range.start => {
                $s.item -= Range::<$t>::delta(range.start, $s.item, 2 as $t).max(1 as $t);
                if $s.item > range.start {
                    Some(Self::new(range.clone(), $s.item))
                } else {
                    None
                }
            }
            Direction::Right if $s.item < range.end => {
                $s.item += Range::<$t>::delta(range.end, $s.item, 2 as $t).max(1 as $t);
                if $s.item < range.end {
                    Some(Self::new(range.clone(), $s.item))
                } else {
                    None
                }
            }
            _ => None,
        }
    }};
}

mod boolean {
    use super::*;

    #[derive(Copy, Clone, Debug, Default)]
    pub struct Shrinker(bool);

    impl FullGenerate for bool {
        type Item = Self;
        type Generate = Full<bool>;
        fn generator() -> Self::Generate {
            Full::new()
        }
    }

    impl Generate for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            let item = state.random.bool();
            (item, Shrinker(item))
        }
    }

    impl Shrink for Shrinker {
        type Item = bool;

        fn generate(&self) -> Self::Item {
            self.0
        }

        fn shrink(&mut self) -> Option<Self> {
            if self.0.change(false) {
                Some(self.clone())
            } else {
                None
            }
        }
    }

    constant!(bool);
}

mod character {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Shrinker(super::Shrinker<u32>);

    impl Range<char> {
        pub fn new(range: impl ops::RangeBounds<char>) -> Result<Self, Error> {
            let start = match range.start_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32)
                    .checked_add(1)
                    .ok_or(Error::Overflow)?
                    .try_into()
                    .map_err(|_| Error::Invalid)?,
                Bound::Unbounded => '\u{0000}',
            };
            let end = match range.end_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32)
                    .checked_sub(1)
                    .ok_or(Error::Overflow)?
                    .try_into()
                    .map_err(|_| Error::Invalid)?,
                Bound::Unbounded if start <= '\u{D7FF}' => '\u{D7FF}',
                Bound::Unbounded if start >= '\u{E000}' => char::MAX,
                Bound::Unbounded => return Err(Error::Invalid),
            };
            if end < start {
                Err(Error::Empty)
            } else {
                Ok(Self { start, end })
            }
        }
    }

    impl Full<char> {
        #[inline]
        const fn low_range() -> Range<char> {
            Range {
                start: '\u{0000}',
                end: '\u{D7FF}',
            }
        }

        #[inline]
        const fn high_range() -> Range<char> {
            Range {
                start: '\u{E000}',
                end: char::MAX,
            }
        }

        #[inline]
        fn shrink(item: char) -> Shrinker {
            let low = Self::low_range();
            let range = if item <= low.end {
                low
            } else {
                Self::high_range()
            };
            Shrinker(super::Shrinker::new(range.into(), item as u32))
        }

        fn special() -> impl Generate<Item = char> {
            const SPECIAL: [char; 3] = ['\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER];
            SPECIAL.any().map(Option::unwrap)
        }
    }

    impl Shrinker {
        #[inline]
        pub fn new(item: char) -> Self {
            let mut low = Full::<char>::low_range();
            let range = if item <= low.end {
                low.end = item;
                low
            } else {
                let mut high = Full::<char>::high_range();
                high.start = item;
                high
            };
            Self(super::Shrinker::new(range.into(), item as u32))
        }
    }

    impl Generate for char {
        type Item = Self;
        type Shrink = Self;

        fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
            (*self, *self)
        }
    }

    impl Shrink for char {
        type Item = Self;

        fn generate(&self) -> Self::Item {
            *self
        }

        fn shrink(&mut self) -> Option<Self> {
            None
        }
    }

    impl Generate for Range<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            let (item, shrink) = Into::<Range<u32>>::into(self.clone()).generate(state);
            (item.try_into().unwrap(), Shrinker(shrink))
        }
    }

    impl Generate for Size<Range<char>> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            let (item, shrink) = Into::<Range<u32>>::into(self.deref().clone())
                .size()
                .generate(state);
            (item.try_into().unwrap(), Shrinker(shrink))
        }
    }

    impl Generate for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            match state.random.u8(..) {
                0..=250 => Self::low_range().generate(state),
                251..=254 => Self::high_range().generate(state),
                255 => {
                    let (item, _) = Self::special().generate(state);
                    (item, Full::<char>::shrink(item))
                }
            }
        }
    }

    impl Generate for Size<Full<char>> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            fn range(range: Range<char>, size: f64, state: &mut State) -> (char, Shrinker) {
                let (item, shrink) = Into::<Range<u32>>::into(range)
                    .shrinked(size.powi(size_of::<char>() as i32))
                    .generate(state);
                (item.try_into().unwrap(), Shrinker(shrink))
            }

            match state.random.u8(..) {
                0..=250 => range(Full::<char>::low_range(), state.size, state),
                251..=254 => range(Full::<char>::high_range(), state.size, state),
                255 => {
                    let (item, _) = Full::<char>::special().generate(state);
                    (item, Full::<char>::shrink(item))
                }
            }
        }
    }

    impl Shrink for Shrinker {
        type Item = char;

        fn generate(&self) -> Self::Item {
            self.0.generate().try_into().unwrap()
        }

        fn shrink(&mut self) -> Option<Self> {
            Some(Self(self.0.shrink()?))
        }
    }

    ranges!(char);
}

mod number {
    use super::*;

    macro_rules! integer {
        ($t:ident) => {
            impl Full<$t> {
                #[inline]
                const fn range() -> Range<$t> {
                    Range { start: $t::MIN, end: $t::MAX }
                }

                #[inline]
                const fn shrink(item: $t) -> Shrinker<$t> {
                    Shrinker::new(Self::range(), item)
                }

                #[inline]
                fn special() -> impl Generate<Item = $t> {
                    const SPECIAL: [$t; 3] = [0 as $t, $t::MIN, $t::MAX];
                    SPECIAL.any().map(Option::unwrap)
                }
            }

            impl Range<$t> {
                pub fn new(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
                    let start = match range.start_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_add(1 as $t).ok_or(Error::Overflow)?,
                        Bound::Unbounded => $t::MIN,
                    };
                    let end = match range.end_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_sub(1 as $t).ok_or(Error::Overflow)?,
                        Bound::Unbounded => $t::MAX,
                    };
                    if end < start {
                        Err(Error::Empty)
                    } else {
                        Ok(Self { start, end })
                    }
                }

                fn delta(left: $t, right: $t, ratio: $t) -> $t {
                    let range = if left < right {
                        (right as u128).wrapping_sub(left as u128)
                    } else {
                        (left as u128).wrapping_sub(right as u128)
                    };
                    (range / ratio as u128) as $t
                }
            }
            shrinked!($t);

            impl Generate for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let item = state.random.$t(self.start..=self.end);
                    (item, Shrinker::new(self.clone(), item))
                }
            }

            impl Shrink for Shrinker<$t> {
                type Item = $t;

                fn generate(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            impl Generate for Size<Range<$t>> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    self.shrinked(state.size).generate(state)
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match state.random.u8(..) {
                        0..=254 => Self::range().generate(state),
                        255 => { let (item, _) = Self::special().generate(state); (item, Self::shrink(item)) },
                    }
                }
            }

            impl Generate for Size<Full<$t>> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match state.random.u8(..) {
                        0..=254 => Full::<$t>::range().shrinked(state.size.powi(size_of::<$t>() as i32)).generate(state),
                        255 => { let (item, _) = Full::<$t>::special().generate(state); (item, Full::<$t>::shrink(item)) },
                    }
                }
            }

            constant!($t);
            ranges!($t);
        };
        ($($ts:ident),*) => { $(integer!($ts);)* };
    }

    macro_rules! floating {
        ($t:ident, $e:expr) => {
            impl Full<$t> {
                fn special() -> impl Generate<Item = $t> {
                    const SPECIAL: [$t; 8] = [0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN];
                    SPECIAL.any().map(Option::unwrap)
                }

                #[inline]
                const fn range() -> Range<$t> {
                    Range { start: $t::MIN, end: $t::MAX }
                }

                #[inline]
                const fn shrink(item: $t) -> Shrinker<$t> {
                    Shrinker::new(Self::range(), item)
                }
            }

            impl Range<$t> {
                pub fn new(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
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
                        Err(Error::Empty)
                    } else if (start.1 || end.1) && start.0 == end.0 {
                        Err(Error::Empty)
                    } else {
                        let epsilon = (end.0 - start.0) * $e;
                        let start = if start.1 { start.0 + epsilon } else { start.0 };
                        let end = if end.1 { end.0 - epsilon } else { end.0 };
                        Ok(Self { start: start.min(end), end: end.max(start) })
                    }
                }

                fn delta(left: $t, right: $t, ratio: $t) -> $t {
                    let range = if left < right {
                        right as f64 - left as f64
                    } else {
                        left as f64 - right as f64
                    };
                    (range / ratio as f64) as $t
                }
            }
            shrinked!($t);

            impl Generate for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let ratio = state.random.$t();
                    let range = self.end - self.start;
                    let item = (range * ratio + self.start).max(self.start).min(self.end);
                    (item, Shrinker::new(self.clone(), item))
                }
            }

            impl Generate for Size<Range<$t>> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    self.shrinked(state.size).generate(state)
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match state.random.u8(..) {
                        0..=126 => Self::range().generate(state),
                        127..=253 => Self::range().map(|value| 1 as $t / value).generate(state),
                        254..=255 => { let (item, _) = Self::special().generate(state); (item, Self::shrink(item)) },
                    }
                }
            }

            impl Generate for Size<Full<$t>> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    fn range(size: f64) -> Range::<$t> {
                        Full::<$t>::range().shrinked(size.powi(size_of::<$t>() as i32))
                    }

                    match state.random.u8(..) {
                        0..=126 => range(state.size).generate(state),
                        127..=253 => range(state.size).map(|value| 1 as $t / value).generate(state),
                        254..=255 => { let (item, _) = Full::<$t>::special().generate(state); (item, Full::<$t>::shrink(item)) },
                    }
                }
            }

            impl Shrink for Shrinker<$t> {
                type Item = $t;

                fn generate(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            constant!($t);
            ranges!($t);
        };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, 3.45266984e-4);
    floating!(f64, 1.4901161193847656e-8);
}
