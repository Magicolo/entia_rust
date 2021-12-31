pub mod all;
pub mod any;
pub mod generator;
pub mod primitive;

use self::any::Any;
pub use generator::{FullGenerator, Generator, IntoGenerator};

#[inline]
pub fn clone<T: Clone>(value: T) -> impl Generator<Item = T> {
    ().map(move |_| value.clone())
}

#[inline]
pub fn default<T: Default>() -> impl Generator<Item = T> {
    ().map(|_| T::default())
}

#[inline]
pub fn letter() -> impl Generator<Item = char> {
    const LETTERS: [char; 52] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    &LETTERS
}

#[inline]
pub fn digit() -> impl Generator<Item = char> {
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    &DIGITS
}

#[inline]
pub fn ascii() -> impl Generator<Item = char> {
    Any::from((
        letter(),
        digit(),
        (0..=0x7Fu8).generator().map(|value| value as char),
    ))
}

#[cfg(test)]
mod tests;
