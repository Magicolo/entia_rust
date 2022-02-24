pub mod all;
pub mod any;
pub mod array;
pub mod collect;
pub mod constant;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod function;
pub mod generator;
pub mod map;
pub mod option;
// pub mod or;
pub mod primitive;
pub mod shrink;
pub mod size;
pub mod wrap;

use self::any::Any;
pub use crate::{
    generator::{FullGenerate, Generate, IntoGenerate},
    shrink::Shrink,
};
pub(crate) use entia_macro::recurse_16 as recurse;
use std::ops::{self, Neg};

pub fn default<T: Default>() -> impl Generate<Item = T> {
    let default: fn() -> T = T::default;
    default
}

pub fn positive<T: Default>() -> impl Generate<Item = T>
where
    ops::RangeFrom<T>: IntoGenerate<Item = T>,
{
    (T::default()..).generator()
}

pub fn negative<T: Neg + Default>() -> impl Generate<Item = T>
where
    ops::RangeTo<T>: IntoGenerate<Item = T>,
{
    (..T::default()).generator()
}

pub fn letter() -> impl Generate<Item = char> {
    const LETTERS: [char; 52] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    Any::from(&LETTERS).map(Option::unwrap)
}

pub fn digit() -> impl Generate<Item = char> {
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    Any::from(&DIGITS).map(Option::unwrap)
}

pub fn ascii() -> impl Generate<Item = char> {
    Any::from((
        letter(),
        digit(),
        (0..=0x7Fu8).generator().map(|value| value as char),
    ))
}

pub fn option<T: FullGenerate>() -> impl Generate<Item = Option<T::Item>> {
    let none: fn() -> Option<T::Item> = || None;
    Any::from((T::generator().map(Some), none))
}

#[cfg(test)]
mod test;
