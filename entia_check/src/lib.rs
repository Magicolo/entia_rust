pub mod all;
pub mod any;
pub mod generator;
pub mod primitive;

use self::{
    any::Any,
    generator::{Count, With},
};
pub use generator::{Generator, IntoGenerator};

pub fn alphabet() -> impl Generator<Item = char> {
    Any(('a'..='z', 'A'..='Z'))
}

pub fn digit() -> impl Generator<Item = char> {
    '0'..='9'
}

pub fn ascii() -> impl Generator<Item = char> {
    Any((
        alphabet(),
        digit(),
        Generator::map(0..=0x7Fu8, |value| value as char),
    ))
}

pub fn string(mut item: impl Generator<Item = char>) -> impl Generator<Item = String> {
    With::new(move |state| {
        Iterator::map(0..Count.generate(state), |_| item.generate(state)).collect()
    })
}

pub fn vector<G: Generator>(mut item: G) -> impl Generator<Item = Vec<G::Item>> {
    With::new(move |state| {
        Iterator::map(0..Count.generate(state), |_| item.generate(state)).collect()
    })
}

#[cfg(test)]
mod tests;
