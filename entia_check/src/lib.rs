pub mod any;
pub mod array;
pub mod check;
pub mod collect;
pub mod constant;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod function;
pub mod generate;
pub mod keep;
pub mod map;
pub mod option;
pub mod primitive;
pub mod sample;
pub mod shrink;
pub mod size;

use self::any::Any;
pub use crate::{
    check::Check,
    generate::{FullGenerate, Generate, IntoGenerate},
    shrink::Shrink,
};
pub(crate) use entia_macro::{tuples_16 as tuples, tuples_with_16 as tuples_with};
use primitive::Range;
use size::Size;
use std::{
    fmt,
    ops::{self, Neg},
};

pub fn default<T: Default>() -> impl Generate<Item = T> {
    let default: fn() -> T = T::default;
    default
}

pub fn number<T>() -> impl Generate<Item = T>
where
    Size<Range<T>>: Generate<Item = T>,
    ops::RangeFull: TryInto<Size<Range<T>>>,
    <ops::RangeFull as TryInto<Size<Range<T>>>>::Error: fmt::Debug,
{
    (..).try_into().unwrap()
}

pub fn positive<T: Default>() -> impl Generate<Item = T>
where
    ops::RangeFrom<T>: IntoGenerate<Item = T>,
{
    (T::default()..).generator()
}

pub fn negative<T: Neg + Default>() -> impl Generate<Item = T>
where
    ops::RangeToInclusive<T>: IntoGenerate<Item = T>,
{
    (..=T::default()).generator()
}

pub fn letter() -> impl Generate<Item = char> {
    const LETTERS: [char; 52] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    LETTERS.any().map(Option::unwrap)
}

pub fn digit() -> impl Generate<Item = char> {
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    DIGITS.any().map(Option::unwrap)
}

pub fn ascii() -> impl Generate<Item = char> {
    Any((
        letter(),
        digit(),
        (0..=0x7Fu8).generator().map(|value| value as char),
    ))
}

pub fn option<T: FullGenerate>() -> impl Generate<Item = Option<T::Item>> {
    let none: fn() -> Option<T::Item> = || None;
    Any((T::generator().map(Some), none))
}

#[cfg(test)]
mod test;
