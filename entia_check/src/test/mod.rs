pub mod character;
pub mod number;

use super::*;
use constant::Constant;

type Result<T> = std::result::Result<(), check::Error<bool, T>>;
const COUNT: usize = 1024;

#[test]
fn boolean_samples_true() {
    assert!(<bool>::generator().sample(COUNT).any(|value| value));
}

#[test]
fn boolean_samples_false() {
    assert!(<bool>::generator().sample(COUNT).any(|value| !value));
}
