pub mod all;
pub mod any;
pub mod generator;
pub mod primitive;

use self::any::Any;
pub(crate) use entia_macro::recurse_16 as recurse;
use generator::constant::{constant, Constant};
pub use generator::{size::Size, FullGenerator, Generator, IntoGenerator};
use std::ops::{self, Neg};

pub fn default<T: Default>() -> impl Generator<Item = T> {
    ().map(|_| T::default())
}

pub fn positive<T: Default>() -> impl Generator<Item = T>
where
    ops::RangeFrom<T>: IntoGenerator<Item = T>,
{
    (T::default()..).generator()
}

pub fn negative<T: Neg + Default>() -> impl Generator<Item = T>
where
    ops::RangeTo<T>: IntoGenerator<Item = T>,
{
    (..T::default()).generator()
}

pub fn letter() -> impl Generator<Item = char> {
    const LETTERS: [Constant<char>; 52] = constant![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'
    ];
    Any::from(&LETTERS).map(Option::unwrap)
}

pub fn digit() -> impl Generator<Item = char> {
    const DIGITS: [Constant<char>; 10] =
        constant!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    Any::from(&DIGITS).map(Option::unwrap)
}

pub fn ascii() -> impl Generator<Item = char> {
    Any::from((
        letter(),
        digit(),
        (0..=0x7Fu8).generator().map(|value| value as char),
    ))
}

pub fn option<T: FullGenerator>() -> impl Generator<Item = Option<T::Item>> {
    let none: fn() -> Option<T::Item> = || None;
    Any::from((T::generator().map(Some), none))
}

#[cfg(test)]
mod test;
